//! Games the Rust host plays (docs/rust/GameHost.md, step D). Java sends `HOST_GAME` instead of starting a game of
//! its own; the gateway starts it on the host and takes the game's calls from then on (`gameJoin`, `sendPlayer*`,
//! `sendPlayerAction`, `matchQuit`, the game chat's). The host's callbacks go to the connection that joined last,
//! numbered and patched as Java's are. At the end the gateway tells Java (`hostGameEnded`), which ends the game and
//! carries on with the table.

use playmat_engine::card::CardDb;
use playmat_host::{Call, Event, HostedGame, SeatStart, Start, Timer};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

/// a connection that gets a game's callbacks: its number, and where its host events go
#[derive(Clone)]
pub struct Subscriber {
    pub connection: u64,
    pub events: UnboundedSender<Value>,
}

pub struct Hosted {
    calls: Mutex<Sender<Call>>,
    /// the human's connections that joined: every tab of theirs gets the game (a closed one drops off)
    subscribers: Mutex<Vec<Subscriber>>,
    /// the spectators' connections (`gameWatchStart`), by user name; they get the board without the human's secrets
    watchers: Mutex<Vec<(Subscriber, String)>>,
    /// the last board as spectators see it, for one who starts watching
    watched: Mutex<Option<Value>>,
    /// the human's player id, whose draws a spectator doesn't see
    human: String,
    pub chat_id: String,
}

impl Hosted {
    pub fn call(&self, call: Call) {
        let _ = self.calls.lock().unwrap().send(call);
    }

    /// a callback for the human's tabs
    pub fn echo(&self, event: Value) {
        self.subscribers.lock().unwrap().retain(|s| s.events.send(event.clone()).is_ok());
    }

    /// a chat line, to everyone in the game's chat: the human's tabs and the spectators
    pub fn say(&self, event: Value) {
        self.to_watchers(event.clone());
        self.echo(event);
    }

    /// a callback for the spectators
    fn to_watchers(&self, event: Value) {
        self.watchers.lock().unwrap().retain(|(s, _)| s.events.send(event.clone()).is_ok());
    }

    /// A host callback: to the human's tabs as it is, and as spectators see it to them
    fn forward(&self, game_id: &str, e: Value) {
        if let Some(w) = watcher_event(game_id, &self.human, &e) {
            if w["method"] != "CHATMESSAGE" && w["method"] != "GAME_OVER" {
                *self.watched.lock().unwrap() = Some(w["data"].clone());
            }
            self.to_watchers(w);
        }
        self.echo(e);
    }

    /// `gameJoin` from a connection: it gets the game's callbacks from now on, beside the human's other tabs
    pub fn subscribe(&self, s: Subscriber) {
        let mut subscribers = self.subscribers.lock().unwrap();
        subscribers.retain(|t| t.connection != s.connection && !t.events.is_closed());
        subscribers.push(s);
    }

    /// `gameWatchStart`: a spectator, sent the board at once. False when the game has no board yet.
    pub fn watch(&self, game_id: &str, s: Subscriber, user: String) -> bool {
        let Some(view) = self.watched.lock().unwrap().clone() else { return false };
        let _ = s.events.send(json!({"objectId": game_id, "method": "GAME_INIT", "data": view}));
        let mut watchers = self.watchers.lock().unwrap();
        watchers.retain(|(t, _)| t.connection != s.connection && !t.events.is_closed());
        watchers.push((s, user));
        true
    }

    pub fn unwatch(&self, connection: u64) {
        self.watchers.lock().unwrap().retain(|(t, _)| t.connection != connection);
    }

    /// `gameWatchers`: the spectators' names
    pub fn watcher_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.watchers.lock().unwrap().iter().map(|(_, u)| u.clone()).collect();
        names.sort();
        names.dedup();
        names
    }
}

/// The board as a spectator sees it (`GameSessionWatcher`: a view for no player): no hand, nothing to play, nothing
/// looked at, and the human's draws hidden
fn watcher_view(human: &str, mut view: Value) -> Value {
    if let Some(v) = view.as_object_mut() {
        v.insert("myHand".into(), json!({}));
        v.insert("myPlayerId".into(), Value::Null);
        v.insert("special".into(), json!(false));
        v.insert("lookedAt".into(), json!([]));
        v.remove("canPlayObjects");
        if let Some(Value::Array(events)) = v.get_mut("events") {
            for e in events.iter_mut() {
                if e["kind"] == "DRAW" && e["playerId"] == human {
                    if let Some(e) = e.as_object_mut() {
                        e.remove("targetId");
                    }
                }
            }
        }
        if let Some(Value::Array(players)) = v.get_mut("players") {
            for p in players {
                p["controlled"] = json!(false);
            }
        }
    }
    view
}

/// A host callback as spectators get it: the board (a question's too, as an update), the log and the end; none for
/// what's only the human's (the questions, their own log lines)
fn watcher_event(game_id: &str, human: &str, e: &Value) -> Option<Value> {
    let method = e["method"].as_str()?;
    let data = &e["data"];
    match method {
        "GAME_INIT" | "GAME_UPDATE" => {
            Some(json!({"objectId": game_id, "method": method, "data": watcher_view(human, data.clone())}))
        }
        "CHATMESSAGE" | "GAME_OVER" | "GAME_ERROR" => Some(e.clone()),
        _ => {
            let view = data.get("gameView").filter(|v| v.is_object())?;
            Some(json!({"objectId": game_id, "method": "GAME_UPDATE", "data": watcher_view(human, view.clone())}))
        }
    }
}

#[derive(Default)]
pub struct Hosts {
    pub enabled: bool,
    games: Mutex<HashMap<String, Arc<Hosted>>>,
    /// each hosted game's chat id, to its game
    chats: Mutex<HashMap<String, String>>,
    /// games started, and ended by how (`end_kind`): step E's gate is a week with no `error` or `panic`
    started: Mutex<u64>,
    /// where each game's record goes (`XMAGE_RUST_HOST_RECORDS`, a folder; playmat-host's `record.rs`), and how many
    /// records are kept (`XMAGE_RUST_HOST_RECORDS_KEEP`, 500 by default; the oldest go first)
    records: Option<PathBuf>,
    keep: usize,
    /// where each game's replay goes (`XMAGE_RUST_HOST_REPLAYS`: Java's `xmage.replays.dir`; playmat-host's
    /// `replay.rs`); none: hosted games have no replay
    replays: Option<PathBuf>,
    ended: Mutex<BTreeMap<&'static str, u64>>,
}

/// Deletes the oldest game records (`*.jsonl`) in `dir` past the newest `keep`
fn prune(dir: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    if files.len() <= keep {
        return;
    }
    files.sort();
    for (_, path) in &files[..files.len() - keep] {
        let _ = std::fs::remove_file(path);
    }
}

/// How a hosted game ended, from its reason: `over` (played out), `conceded` (the game, or the match: `quit`),
/// `timed out` (a player's priority timer ran out), `stopped` (no answer for 30 minutes, or the player left), `error`
/// (the host stopped on an error, or couldn't start) or `panic`
pub fn end_kind(reason: &str) -> &'static str {
    match reason {
        "over" => "over",
        "conceded" | "quit" => "conceded",
        "timed out" => "timed out",
        "stopped" => "stopped",
        r if r.starts_with("error") => "error",
        _ => "panic",
    }
}

fn db() -> Arc<CardDb> {
    static DB: OnceLock<Arc<CardDb>> = OnceLock::new();
    DB.get_or_init(|| Arc::new(playmat_engine::cards())).clone()
}

impl Hosts {
    pub fn new(enabled: bool) -> Hosts {
        if enabled {
            // the card pool loads in the background, so the first game doesn't wait for it
            std::thread::spawn(db);
        }
        let records = std::env::var_os("XMAGE_RUST_HOST_RECORDS").filter(|v| !v.is_empty()).map(PathBuf::from);
        let keep = std::env::var("XMAGE_RUST_HOST_RECORDS_KEEP").ok().and_then(|v| v.parse().ok()).unwrap_or(500);
        if let (true, Some(dir)) = (enabled, &records) {
            info!("Rust host: game records in {} (the last {keep})", dir.display());
        }
        let replays = std::env::var_os("XMAGE_RUST_HOST_REPLAYS").filter(|v| !v.is_empty()).map(PathBuf::from);
        Hosts { enabled, records, keep, replays, ..Hosts::default() }
    }

    pub fn get(&self, game_id: &str) -> Option<Arc<Hosted>> {
        self.games.lock().unwrap().get(game_id).cloned()
    }

    pub fn game_of_chat(&self, chat_id: &str) -> Option<String> {
        self.chats.lock().unwrap().get(chat_id).cloned()
    }

    pub fn count(&self) -> usize {
        self.games.lock().unwrap().len()
    }

    fn ended(&self, reason: &str) {
        *self.ended.lock().unwrap().entry(end_kind(reason)).or_default() += 1;
    }

    /// The counters for `/metrics`: games playing, started, and ended by how
    pub fn render(&self, out: &mut String) {
        let _ = writeln!(out, "# TYPE playmat_gateway_host_games_open gauge");
        let _ = writeln!(out, "playmat_gateway_host_games_open {}", self.count());
        let _ = writeln!(out, "# TYPE playmat_gateway_host_games_started_total counter");
        let _ = writeln!(out, "playmat_gateway_host_games_started_total {}", self.started.lock().unwrap());
        let _ = writeln!(out, "# TYPE playmat_gateway_host_games_ended_total counter");
        let ended = self.ended.lock().unwrap();
        for how in ["over", "conceded", "timed out", "stopped", "error", "panic"] {
            let n = ended.get(how).copied().unwrap_or(0);
            let _ = writeln!(out, "playmat_gateway_host_games_ended_total{{how=\"{how}\"}} {n}");
        }
    }

    /// Java's `HOST_GAME`: starts the game; its callbacks go to `subscriber`. The end is reported to Java at
    /// `java_url`.
    pub fn start(self: &Arc<Self>, event: &Value, subscriber: Subscriber, java_url: url::Url) {
        let Some(game_id) = event["objectId"].as_str().map(str::to_string) else { return };
        let data = &event["data"];
        let token = data["token"].as_str().unwrap_or("").to_string();
        let seed = data["seed"].as_i64().unwrap_or(0);
        let seats: Vec<SeatStart> = data["seats"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| SeatStart {
                        player_id: s["playerId"].as_str().unwrap_or("").to_string(),
                        name: s["name"].as_str().unwrap_or("").to_string(),
                        human: s["human"].as_bool().unwrap_or(false),
                        deck: s["deck"]
                            .as_array()
                            .map(|d| d.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
                            .unwrap_or_default(),
                        printings: s["printings"]
                            .as_array()
                            .map(|d| d.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
                            .unwrap_or_default(),
                        user_data: s.get("userData").cloned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let players: Vec<String> = seats.iter().map(|s| s.player_id.clone()).collect();
        let secs = |v: &Value| v.as_u64().and_then(|n| u32::try_from(n).ok());
        let mut timer = Timer {
            priority_secs: secs(&data["priorityTime"]).unwrap_or(0),
            buffer_secs: secs(&data["bufferTime"]).unwrap_or(0),
            ..Timer::default()
        };
        // a match's later game: each seat's time left from the earlier ones
        for (seat, s) in data["seats"].as_array().into_iter().flatten().take(2).enumerate() {
            timer.left[seat] = secs(&s["priorityTimeLeft"]);
        }
        // a match's next game: its last loser chooses who plays first
        let chooser = data["chooser"].as_str().filter(|c| !c.is_empty()).map(str::to_string);
        let record = self.records.as_ref().map(|dir| {
            prune(dir, self.keep.saturating_sub(1));
            dir.join(format!("{game_id}.jsonl"))
        });
        let rules = playmat_host::TableRules::of(data);
        // the replay goes where Java keeps its games' (XMAGE_RUST_HOST_REPLAYS: its xmage.replays.dir), listed at the end
        let replay = self.replays.as_ref().map(|dir| dir.join(format!("{game_id}.ndjson.gz.part")));
        let start = Start {
            game_id: game_id.clone(),
            seed,
            seats,
            rules,
            timer,
            chooser,
            record,
            replay,
            log_to: None,
            drop_stale: true,
        };
        // known before the game starts, so its calls (the client joins at once) wait for it
        let chat_id = playmat_host::ids::Ids::new(&game_id, players.clone()).chat();
        let (calls, calls_in) = std::sync::mpsc::channel();
        let human = data["seats"]
            .as_array()
            .and_then(|a| a.iter().find(|s| s["human"] == true))
            .and_then(|s| s["playerId"].as_str())
            .unwrap_or("")
            .to_string();
        let hosted = Arc::new(Hosted {
            calls: Mutex::new(calls.clone()),
            subscribers: Mutex::new(vec![subscriber.clone()]),
            watchers: Mutex::new(Vec::new()),
            watched: Mutex::new(None),
            human,
            chat_id: chat_id.clone(),
        });
        self.games.lock().unwrap().insert(game_id.clone(), hosted.clone());
        *self.started.lock().unwrap() += 1;
        self.chats.lock().unwrap().insert(chat_id.clone(), game_id.clone());
        let handle = tokio::runtime::Handle::current();
        let hosts = self.clone();
        // a thread of its own: it waits on the game's events
        std::thread::Builder::new()
            .name(format!("host events {game_id}"))
            .spawn(move || {
                let game = match HostedGame::start_with(start, db(), (calls, calls_in)) {
                    Ok(g) => g,
                    Err(why) => {
                        hosts.games.lock().unwrap().remove(&game_id);
                        hosts.chats.lock().unwrap().remove(&chat_id);
                        warn!("Rust host: game {game_id} can't start: {why}");
                        let over = json!({"objectId": game_id, "method": "GAME_OVER",
                            "data": {"message": format!("The game couldn't start on the Rust host: {why}")}});
                        let _ = subscriber.events.send(over);
                        hosts.ended("error: can't start");
                        let end = Report {
                            winner: None,
                            turns: 0,
                            reason: format!("error: {why}"),
                            time_left: Value::Null,
                            summary: Value::Null,
                        };
                        handle.spawn(report(java_url, game_id, token, end));
                        return;
                    }
                };
                let (_, events, thread) = game.split();
                info!("Rust host: game {game_id} started");
                while let Ok(e) = events.recv() {
                    let e: Event = e;
                    hosted.forward(&game_id, e.json());
                }
                let ended = thread.join().unwrap_or(playmat_host::Ended {
                    reason: "panic: the host's thread".into(),
                    ..playmat_host::Ended::default()
                });
                hosts.games.lock().unwrap().remove(&game_id);
                hosts.chats.lock().unwrap().remove(&chat_id);
                let winner = ended.winner.and_then(|w| players.get(w).cloned());
                info!("Rust host: game {game_id} ended after {} turns: {}", ended.turns, ended.reason);
                hosts.ended(&ended.reason);
                let time_left = time_left(&players, &ended.time_left);
                let end =
                    Report { winner, turns: ended.turns, reason: ended.reason, time_left, summary: ended.summary };
                handle.spawn(report(java_url, game_id, token, end));
            })
            .expect("a thread");
    }
}

/// Each player's priority time left, by player id, for the match's next game; null without a timer
fn time_left(players: &[String], left: &[Option<u32>; 2]) -> Value {
    let map: serde_json::Map<String, Value> =
        players.iter().zip(left).filter_map(|(id, l)| l.map(|secs| (id.clone(), json!(secs)))).collect();
    if map.is_empty() {
        Value::Null
    } else {
        Value::Object(map)
    }
}

/// How a hosted game ended, for Java (`hostGameEnded`'s params after the game and the token)
struct Report {
    winner: Option<String>,
    turns: u32,
    reason: String,
    time_left: Value,
    summary: Value,
}

/// `hostGameEnded` to Java, on a connection of its own (the player's may be gone); a few tries
async fn report(java_url: url::Url, game_id: String, token: String, end: Report) {
    use futures_util::{SinkExt, StreamExt};
    let Report { winner, turns, reason, time_left, summary } = end;
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "hostGameEnded",
        "params": [game_id, token, winner.unwrap_or_default(), turns, reason, time_left, summary]});
    for attempt in 0..5u64 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(2 * attempt)).await;
        }
        let Ok(mut ws) = yawc::WebSocket::connect(java_url.clone()).await else { continue };
        if ws.send(yawc::Frame::text(bytes::Bytes::from(request.to_string()))).await.is_err() {
            continue;
        }
        while let Some(frame) = ws.next().await {
            if frame.opcode() != yawc::OpCode::Text {
                continue;
            }
            let Ok(reply) = serde_json::from_str::<Value>(frame.as_str()) else { continue };
            if reply.get("id") == Some(&json!(1)) {
                if reply.get("result") != Some(&Value::Bool(true)) {
                    warn!("Rust host: Java didn't take game {}'s end: {reply}", request["params"][0]);
                }
                let _ = ws.close().await;
                return;
            }
        }
    }
    warn!("Rust host: couldn't tell Java game {} ended", request["params"][0]);
}

/// A player's chat line as Java's `CHATMESSAGE` (`TALK`, blue, HTML escaped, as `HtmlEscape.escapeHtml4`)
pub fn talk(chat_id: &str, user: &str, message: &str) -> Value {
    let mut escaped = String::with_capacity(message.len());
    for c in message.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            c => escaped.push(c),
        }
    }
    let time =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    json!({"objectId": chat_id, "method": "CHATMESSAGE", "data": {"username": user, "message": escaped, "time": time,
        "color": "BLUE", "messageType": "TALK"}})
}

/// The game a call is about, and where in its params the game id is, for the calls the host takes
pub fn game_param(method: &str) -> Option<usize> {
    Some(match method {
        "gameJoin" | "gameResync" | "gameStateResync" | "matchQuit" | "sendPlayerUUID" | "sendPlayerString"
        | "sendPlayerBoolean" | "sendPlayerInteger" | "sendPlayerManaType" | "chatFindByGame" | "gameWatchStart"
        | "gameWatchStop" | "gameWatchers" => 0,
        "sendPlayerAction" => 1,
        _ => return None,
    })
}

/// A game call as the host's: None for one it only acknowledges
pub fn call_of(method: &str, params: &[Value]) -> Option<Call> {
    let p = |i: usize| params.get(i).cloned().unwrap_or(Value::Null);
    Some(match method {
        "gameJoin" => Call::Join,
        "gameResync" | "gameStateResync" => Call::Resync,
        "matchQuit" => Call::Quit,
        "sendPlayerUUID" => Call::Uuid(p(2).as_str().map(str::to_string)),
        "sendPlayerString" => Call::Str(p(2).as_str().unwrap_or("").to_string()),
        "sendPlayerBoolean" => Call::Bool(p(2).as_bool().unwrap_or(false)),
        "sendPlayerInteger" => Call::Int(p(2).as_i64().unwrap_or(0) as i32),
        "sendPlayerManaType" => Call::ManaType(p(3).as_str().unwrap_or("").to_string()),
        "sendPlayerAction" => {
            let data = p(3);
            Call::Action(p(0).as_str().unwrap_or("").to_string(), (!data.is_null()).then_some(data))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_keeps_the_newest() {
        let dir = std::env::temp_dir().join(format!("playmat-prune-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..5 {
            std::fs::write(dir.join(format!("g{i}.jsonl")), "{}").unwrap();
            // distinct times, oldest first
            let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000 + i);
            std::fs::File::options()
                .write(true)
                .open(dir.join(format!("g{i}.jsonl")))
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        std::fs::write(dir.join("other.txt"), "").unwrap();
        prune(&dir, 2);
        let mut left: Vec<String> =
            std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["g3.jsonl", "g4.jsonl", "other.txt"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_metrics() {
        assert_eq!(end_kind("over"), "over");
        assert_eq!(end_kind("quit"), "conceded");
        assert_eq!(end_kind("timed out"), "timed out");
        let players = vec!["a".to_string(), "b".to_string()];
        assert_eq!(time_left(&players, &[Some(40), Some(0)]), json!({"a": 40, "b": 0}));
        assert_eq!(time_left(&players, &[None, None]), Value::Null);
        assert_eq!(end_kind("error: no such card"), "error");
        assert_eq!(end_kind("panic: the host's thread"), "panic");
        let hosts = Hosts::default();
        hosts.ended("over");
        hosts.ended("over");
        hosts.ended("error: x");
        let mut out = String::new();
        hosts.render(&mut out);
        assert!(out.contains("playmat_gateway_host_games_ended_total{how=\"over\"} 2"));
        assert!(out.contains("playmat_gateway_host_games_ended_total{how=\"error\"} 1"));
        assert!(out.contains("playmat_gateway_host_games_ended_total{how=\"panic\"} 0"));
        assert!(out.contains("playmat_gateway_host_games_open 0"));
    }

    #[test]
    fn a_chat_line_is_escaped_as_javas() {
        let e = talk("c", "ann", "<b>hi</b> & 'bye'");
        assert_eq!(e["method"], "CHATMESSAGE");
        assert_eq!(e["data"]["message"], "&lt;b&gt;hi&lt;/b&gt; &amp; &#39;bye&#39;");
        assert_eq!(e["data"]["messageType"], "TALK");
        assert_eq!(game_param("gameEmote"), None);
    }

    #[test]
    fn a_spectator_sees_no_hand() {
        let view = json!({"myHand": {"c1": {}}, "myPlayerId": "h", "canPlayObjects": {"objects": {}}, "lookedAt": [{}],
            "players": [{"playerId": "h", "controlled": true}],
            "events": [{"kind": "DRAW", "playerId": "h", "targetId": "c2"}, {"kind": "DRAW", "playerId": "a"}]});
        let ask = json!({"objectId": "g", "method": "GAME_TARGET", "data": {"message": "?", "gameView": view}});
        let w = watcher_event("g", "h", &ask).unwrap();
        assert_eq!(w["method"], "GAME_UPDATE", "a question is an update to a spectator");
        let v = &w["data"];
        assert_eq!(v["myHand"], json!({}));
        assert!(v.get("canPlayObjects").is_none());
        assert_eq!(v["myPlayerId"], Value::Null);
        assert_eq!(v["players"][0]["controlled"], false);
        assert!(v["events"][0].get("targetId").is_none(), "the human's draw is hidden");
        let only_human = json!({"objectId": "g", "method": "GAME_INFORM_PERSONAL", "data": {"message": "x"}});
        assert!(watcher_event("g", "h", &only_human).is_none());
    }
}
