//! Game-state patches made in the gateway instead of the Java bridge: a port of `mage.server.websocket.GameStates`
//! (docs/WebSocketAPI.md, "State patches"). Java sends every view complete; for a client that asked for
//! `stateDiffs`, the gateway keeps the last view of each game and sends patches against it.

use playmat_statediff::diff;
use serde_json::{Map, Value};

/// games remembered per connection; an older game's next state goes out complete
pub const MAX_GAMES: usize = 8;
/// every so many patches, the complete message is written again to keep its size current
pub const PATCHES_PER_SIZE_CHECK: u32 = 16;

const VIEW_IS_DATA: [&str; 4] = ["GAME_INIT", "GAME_UPDATE", "REPLAY_INIT", "REPLAY_UPDATE"];
const REPLAYS: [&str; 2] = ["REPLAY_INIT", "REPLAY_UPDATE"];
/// the game is over: the client gets everything and both sides forget it
const LAST: [&str; 2] = ["GAME_OVER", "REPLAY_DONE"];

struct Baseline {
    seq: u64,
    view: Value,
    replay: bool,
    full_chars: usize,
    patches_since_size_check: u32,
}

/// One message, ready to send.
pub struct Encoded {
    pub json: String,
    /// size of the complete message, what a client without patches would have received
    pub full_chars: usize,
    pub patch: bool,
}

#[derive(Default)]
pub struct GameStates {
    /// least recently used first
    games: Vec<(String, Baseline)>,
    next_seq: u64,
    enabled: bool,
    last_message_id: i64,
}

impl GameStates {
    pub fn new() -> GameStates {
        GameStates { games: Vec::new(), next_seq: 1, enabled: false, last_message_id: -1 }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// turned on by the client; turning it off forgets every game
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.games.clear();
        }
    }

    pub fn sent(&mut self, message_id: i64) {
        self.last_message_id = self.last_message_id.max(message_id);
    }

    /// a new login on the connection: message ids start again
    pub fn new_session(&mut self) {
        self.games.clear();
        self.last_message_id = -1;
    }

    pub fn forget(&mut self, game_id: &str) {
        self.games.retain(|(id, _)| id != game_id);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.games.len()
    }

    fn take(&mut self, game_id: &str) -> Option<Baseline> {
        let index = self.games.iter().position(|(id, _)| id == game_id)?;
        Some(self.games.remove(index).1)
    }

    fn put(&mut self, game_id: &str, baseline: Baseline) {
        self.forget(game_id);
        self.games.push((game_id.to_string(), baseline));
        if self.games.len() > MAX_GAMES {
            self.games.remove(0);
        }
    }

    /// `message` is a server event; None when it holds no game view (send it as it is)
    pub fn encode(&mut self, message: &Map<String, Value>) -> Option<Encoded> {
        let method = message.get("method")?.as_str()?;
        let game_id = message.get("objectId")?.as_str()?.to_string();
        if LAST.contains(&method) {
            self.forget(&game_id);
            return None;
        }
        let data = message.get("data")?.as_object()?;
        let view_is_data = VIEW_IS_DATA.contains(&method);
        let view = if view_is_data { message.get("data")? } else { data.get("gameView").filter(|v| v.is_object())? };

        let seq = self.next_seq;
        self.next_seq += 1;
        let replay = REPLAYS.contains(&method);
        let previous = self.take(&game_id);
        let patch = previous.as_ref().and_then(|p| diff(&p.view, view));
        let whole = matches!(&patch, Some(Value::Array(a)) if a.len() == 1);
        let Some(previous) = previous.filter(|_| !whole) else {
            // the first state, or one replaced as a whole: nothing to patch
            let full = complete(message, seq);
            let full_chars = full.len();
            self.put(&game_id, Baseline { seq, view: view.clone(), replay, full_chars, patches_since_size_check: 0 });
            return Some(Encoded { json: full, full_chars, patch: false });
        };

        let mut state = Map::new();
        state.insert("seq".into(), Value::from(seq));
        state.insert("base".into(), Value::from(previous.seq));
        state.insert("patch".into(), patch.unwrap_or_else(|| Value::Object(Map::new())));
        let patched = Value::Object(without_view(message, view_is_data, state)).to_string();
        if patched.len() * 2 < previous.full_chars && previous.patches_since_size_check + 1 < PATCHES_PER_SIZE_CHECK {
            let full_chars = previous.full_chars;
            self.put(
                &game_id,
                Baseline {
                    seq,
                    view: view.clone(),
                    replay,
                    full_chars,
                    patches_since_size_check: previous.patches_since_size_check + 1,
                },
            );
            return Some(Encoded { json: patched, full_chars, patch: true });
        }
        // a close call, or time to measure again: write the complete message too, and send the smaller
        let full = complete(message, seq);
        let full_chars = full.len();
        self.put(&game_id, Baseline { seq, view: view.clone(), replay, full_chars, patches_since_size_check: 0 });
        Some(if patched.len() < full.len() {
            Encoded { json: patched, full_chars, patch: true }
        } else {
            Encoded { json: full, full_chars, patch: false }
        })
    }

    /// The client lost track of a game: its last state, complete, as an update event. None when nothing is
    /// remembered for the game.
    pub fn resync(&mut self, game_id: &str) -> Option<String> {
        let baseline = self.take(game_id)?;
        let seq = self.next_seq;
        self.next_seq += 1;
        let mut message = Map::new();
        message.insert("method".into(), Value::from(if baseline.replay { "REPLAY_UPDATE" } else { "GAME_UPDATE" }));
        message.insert("objectId".into(), Value::from(game_id));
        message.insert("messageId".into(), Value::from(self.last_message_id.max(0)));
        message.insert("data".into(), baseline.view.clone());
        let mut state = Map::new();
        state.insert("seq".into(), Value::from(seq));
        message.insert("state".into(), Value::Object(state));
        let text = Value::Object(message).to_string();
        self.put(game_id, Baseline { seq, ..baseline });
        Some(text)
    }
}

/// the message with its view, as text
fn complete(message: &Map<String, Value>, seq: u64) -> String {
    let mut copy = message.clone();
    copy.remove("state");
    let mut state = Map::new();
    state.insert("seq".into(), Value::from(seq));
    copy.insert("state".into(), Value::Object(state));
    Value::Object(copy).to_string()
}

/// the message without its view (the client rebuilds it from the patch in `state`)
fn without_view(message: &Map<String, Value>, view_is_data: bool, state: Map<String, Value>) -> Map<String, Value> {
    let mut copy = Map::new();
    for (key, value) in message {
        if key == "state" {
            continue;
        }
        if key != "data" {
            copy.insert(key.clone(), value.clone());
        } else if !view_is_data {
            let mut data = Map::new();
            if let Some(fields) = value.as_object() {
                for (field, value) in fields {
                    if field != "gameView" {
                        data.insert(field.clone(), value.clone());
                    }
                }
            }
            copy.insert(key.clone(), Value::Object(data));
        }
    }
    copy.insert("state".into(), Value::Object(state));
    copy
}

#[cfg(test)]
mod tests {
    use super::*;
    use playmat_statediff::apply;
    use serde_json::json;

    fn event(method: &str, view: Value) -> Map<String, Value> {
        json!({"method": method, "objectId": "g1", "messageId": 3, "data": view}).as_object().unwrap().clone()
    }

    fn big_view(life: i64) -> Value {
        let cards: Vec<Value> = (0..50).map(|i| json!({"name": format!("Card {i}"), "power": i})).collect();
        json!({"turn": 1, "life": life, "cards": cards})
    }

    #[test]
    fn first_state_complete_then_patches() {
        let mut states = GameStates::new();
        let first = states.encode(&event("GAME_UPDATE", big_view(20))).unwrap();
        assert!(!first.patch);
        let first: Value = serde_json::from_str(&first.json).unwrap();
        assert_eq!(first["state"], json!({"seq": 1}));

        let second = states.encode(&event("GAME_UPDATE", big_view(18))).unwrap();
        assert!(second.patch);
        let second: Value = serde_json::from_str(&second.json).unwrap();
        assert!(second.get("data").is_none());
        assert_eq!(second["state"]["base"], json!(1));
        let rebuilt = apply(Some(&first["data"]), Some(&second["state"]["patch"])).unwrap();
        assert_eq!(rebuilt, big_view(18));
    }

    #[test]
    fn prompts_carry_the_view_in_game_view() {
        let mut states = GameStates::new();
        states.encode(&event("GAME_UPDATE", big_view(20)));
        let ask = json!({"method": "GAME_ASK", "objectId": "g1", "messageId": 4, "data": {"message": "Mulligan?", "gameView": big_view(19)}});
        let encoded = states.encode(ask.as_object().unwrap()).unwrap();
        let ask: Value = serde_json::from_str(&encoded.json).unwrap();
        assert!(encoded.patch);
        assert_eq!(ask["data"], json!({"message": "Mulligan?"}));
    }

    #[test]
    fn game_over_forgets_and_resync_sends_the_last_state() {
        let mut states = GameStates::new();
        states.encode(&event("GAME_UPDATE", big_view(20)));
        states.sent(7);
        let resync: Value = serde_json::from_str(&states.resync("g1").unwrap()).unwrap();
        assert_eq!(resync["method"], json!("GAME_UPDATE"));
        assert_eq!(resync["messageId"], json!(7));
        assert_eq!(resync["data"], big_view(20));
        let over = json!({"method": "GAME_OVER", "objectId": "g1", "messageId": 8, "data": {"message": "done"}});
        assert!(states.encode(over.as_object().unwrap()).is_none());
        assert!(states.resync("g1").is_none());
    }

    #[test]
    fn remembers_eight_games() {
        let mut states = GameStates::new();
        for i in 0..10 {
            let mut e = event("GAME_UPDATE", big_view(20));
            e.insert("objectId".into(), json!(format!("g{i}")));
            states.encode(&e);
        }
        assert_eq!(states.len(), MAX_GAMES);
        assert!(states.resync("g0").is_none());
        assert!(states.resync("g9").is_some());
    }
}
