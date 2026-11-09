//! Games the Rust host plays (docs/rust/GameHost.md, step D). Java sends `HOST_GAME` instead of starting a game of
//! its own; the gateway starts it on the host and takes the game's calls from then on (`gameJoin`, `sendPlayer*`,
//! `sendPlayerAction`, `matchQuit`, the game chat's). The host's callbacks go to the connection that joined last,
//! numbered and patched as Java's are. At the end the gateway tells Java (`hostGameEnded`), which ends the game and
//! carries on with the table.

use playmat_engine::card::CardDb;
use playmat_host::{Call, Event, HostedGame, SeatStart, Start};
use serde_json::{json, Value};
use std::collections::HashMap;
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
    subscriber: Mutex<Option<Subscriber>>,
    pub chat_id: String,
}

impl Hosted {
    pub fn call(&self, call: Call) {
        let _ = self.calls.lock().unwrap().send(call);
    }

    pub fn subscribe(&self, s: Subscriber) {
        *self.subscriber.lock().unwrap() = Some(s);
    }
}

#[derive(Default)]
pub struct Hosts {
    pub enabled: bool,
    games: Mutex<HashMap<String, Arc<Hosted>>>,
    /// each hosted game's chat id, to its game
    chats: Mutex<HashMap<String, String>>,
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
        Hosts { enabled, ..Hosts::default() }
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
                        user_data: s.get("userData").cloned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let players: Vec<String> = seats.iter().map(|s| s.player_id.clone()).collect();
        let start = Start { game_id: game_id.clone(), seed, seats };
        // known before the game starts, so its calls (the client joins at once) wait for it
        let chat_id = playmat_host::ids::Ids::new(&game_id, players.clone()).chat();
        let (calls, calls_in) = std::sync::mpsc::channel();
        let hosted = Arc::new(Hosted {
            calls: Mutex::new(calls.clone()),
            subscriber: Mutex::new(Some(subscriber.clone())),
            chat_id: chat_id.clone(),
        });
        self.games.lock().unwrap().insert(game_id.clone(), hosted.clone());
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
                        handle.spawn(report(java_url, game_id, token, None, 0, format!("error: {why}")));
                        return;
                    }
                };
                let (_, events, thread) = game.split();
                info!("Rust host: game {game_id} started");
                while let Ok(e) = events.recv() {
                    let e: Event = e;
                    let to = hosted.subscriber.lock().unwrap().clone();
                    if let Some(to) = to {
                        let _ = to.events.send(e.json());
                    }
                }
                let ended = thread.join().unwrap_or(playmat_host::Ended {
                    winner: None,
                    turns: 0,
                    reason: "the host's thread panicked".into(),
                });
                hosts.games.lock().unwrap().remove(&game_id);
                hosts.chats.lock().unwrap().remove(&chat_id);
                let winner = ended.winner.and_then(|w| players.get(w).cloned());
                info!("Rust host: game {game_id} ended after {} turns: {}", ended.turns, ended.reason);
                handle.spawn(report(java_url, game_id, token, winner, ended.turns, ended.reason));
            })
            .expect("a thread");
    }
}

/// `hostGameEnded` to Java, on a connection of its own (the player's may be gone); a few tries
async fn report(
    java_url: url::Url,
    game_id: String,
    token: String,
    winner: Option<String>,
    turns: u32,
    reason: String,
) {
    use futures_util::{SinkExt, StreamExt};
    let request = json!({"jsonrpc": "2.0", "id": 1, "method": "hostGameEnded",
        "params": [game_id, token, winner.unwrap_or_default(), turns, reason]});
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

/// The game a call is about, and where in its params the game id is, for the calls the host takes
pub fn game_param(method: &str) -> Option<usize> {
    Some(match method {
        "gameJoin" | "gameResync" | "gameStateResync" | "matchQuit" | "sendPlayerUUID" | "sendPlayerString"
        | "sendPlayerBoolean" | "sendPlayerInteger" | "sendPlayerManaType" | "gameEmote" | "chatFindByGame" => 0,
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
