//! A hosted game's record (docs/rust/GameHost.md): its start, every call the human's client sent in the order the host
//! took them, the game log, and the end, one JSON object a line. The engine is deterministic and the AI's search is
//! bounded by answers, not time, so the start and the calls play the same game again ([`replay`]); the client's ids
//! are made from the game's id and the engine's objects, so the recorded calls name the same objects.
//!
//! ```text
//! {"start": {"gameId", "seed", "seats": [{"playerId", "name", "human", "deck", "userData"}], "timer", "chooser"}}
//! {"call": {"uuid": "…"}, "turn": 3, "ref": "H0.4"}
//! {"log": "<font …>A</font> casts Fatal Push targeting …", "turn": 3}
//! {"end": {"winner": 0, "turns": 9, "reason": "over"}}
//! ```

use crate::game::{Call, Ended, HostedGame, SeatStart, Start};
use crate::timer::Timer;
use playmat_engine::card::CardDb;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Where a game's record goes, a line at a time (flushed, so a gateway that stops keeps what was played)
pub struct Recorder {
    file: File,
}

impl Recorder {
    pub fn create(path: &Path, start: &Start) -> std::io::Result<Recorder> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut r = Recorder { file: File::create(path)? };
        r.line(json!({"start": start_json(start)}));
        Ok(r)
    }

    pub fn line(&mut self, v: Value) {
        // a record that can't be written never stops the game
        let _ = writeln!(self.file, "{v}");
        let _ = self.file.flush();
    }

    pub fn call(&mut self, call: &Call, turn: u32, reference: Option<&str>) {
        let mut v = json!({"call": call_json(call), "turn": turn});
        if let Some(r) = reference {
            v["ref"] = json!(r);
        }
        self.line(v);
    }

    pub fn end(&mut self, ended: &Ended) {
        self.line(json!({"end": {"winner": ended.winner, "turns": ended.turns, "reason": ended.reason}}));
    }
}

fn start_json(s: &Start) -> Value {
    json!({
        "gameId": s.game_id,
        "seed": s.seed,
        "seats": s.seats.iter().map(|p| json!({
            "playerId": p.player_id, "name": p.name, "human": p.human, "deck": p.deck, "printings": p.printings, "userData": p.user_data,
        })).collect::<Vec<_>>(),
        "timer": {"prioritySecs": s.timer.priority_secs, "bufferSecs": s.timer.buffer_secs, "left": s.timer.left},
        "chooser": s.chooser,
    })
}

fn start_of(v: &Value) -> Option<Start> {
    let t = &v["timer"];
    let left = |i: usize| t["left"][i].as_u64().map(|n| n as u32);
    Some(Start {
        game_id: v["gameId"].as_str()?.to_string(),
        seed: v["seed"].as_i64()?,
        seats: v["seats"]
            .as_array()?
            .iter()
            .map(|p| SeatStart {
                player_id: p["playerId"].as_str().unwrap_or("").to_string(),
                name: p["name"].as_str().unwrap_or("").to_string(),
                human: p["human"].as_bool().unwrap_or(false),
                deck: p["deck"]
                    .as_array()
                    .map(|d| d.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                printings: p["printings"]
                    .as_array()
                    .map(|d| d.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                user_data: p.get("userData").filter(|u| !u.is_null()).cloned(),
            })
            .collect(),
        timer: Timer {
            priority_secs: t["prioritySecs"].as_u64().unwrap_or(0) as u32,
            buffer_secs: t["bufferSecs"].as_u64().unwrap_or(0) as u32,
            left: [left(0), left(1)],
        },
        chooser: v["chooser"].as_str().map(str::to_string),
        record: None,
        log_to: None,
    })
}

pub fn call_json(c: &Call) -> Value {
    match c {
        Call::Join => json!({"join": true}),
        Call::Uuid(u) => json!({"uuid": u}),
        Call::Bool(b) => json!({"bool": b}),
        Call::Int(n) => json!({"int": n}),
        Call::Str(s) => json!({"str": s}),
        Call::ManaType(m) => json!({"manaType": m}),
        Call::Action(a, d) => json!({"action": a, "data": d}),
        Call::Resync => json!({"resync": true}),
        Call::Quit => json!({"quit": true}),
    }
}

pub fn call_of(v: &Value) -> Option<Call> {
    let o = v.as_object()?;
    Some(if o.contains_key("join") {
        Call::Join
    } else if let Some(u) = o.get("uuid") {
        Call::Uuid(u.as_str().map(str::to_string))
    } else if let Some(b) = o.get("bool") {
        Call::Bool(b.as_bool()?)
    } else if let Some(n) = o.get("int") {
        Call::Int(n.as_i64()? as i32)
    } else if let Some(s) = o.get("str") {
        Call::Str(s.as_str()?.to_string())
    } else if let Some(m) = o.get("manaType") {
        Call::ManaType(m.as_str()?.to_string())
    } else if let Some(a) = o.get("action") {
        Call::Action(a.as_str()?.to_string(), o.get("data").filter(|d| !d.is_null()).cloned())
    } else if o.contains_key("resync") {
        Call::Resync
    } else if o.contains_key("quit") {
        Call::Quit
    } else {
        return None;
    })
}

/// A record read back
pub struct Recorded {
    pub start: Start,
    pub calls: Vec<Call>,
    /// the game log's lines, with the turn each was written in
    pub log: Vec<(u32, String)>,
    /// `{winner, turns, reason}`; none when the record stops before the end
    pub end: Option<Value>,
}

pub fn read(path: &Path) -> Result<Recorded, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut start = None;
    let mut calls = Vec::new();
    let mut log = Vec::new();
    let mut end = None;
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line).map_err(|e| format!("line {}: {e}", i + 1))?;
        if let Some(s) = v.get("start") {
            start = Some(start_of(s).ok_or(format!("line {}: a start without its fields", i + 1))?);
        } else if let Some(c) = v.get("call") {
            calls.push(call_of(c).ok_or(format!("line {}: an unknown call", i + 1))?);
        } else if let Some(l) = v.get("log").and_then(Value::as_str) {
            log.push((v["turn"].as_u64().unwrap_or(0) as u32, l.to_string()));
        } else if let Some(e) = v.get("end") {
            end = Some(e.clone());
        }
    }
    Ok(Recorded { start: start.ok_or("no start line")?, calls, log, end })
}

/// What a replay played: the log, how it ended, and the first log line that differs from the record's
pub struct Replayed {
    pub log: Vec<(u32, String)>,
    pub ended: Ended,
    /// (index, recorded, replayed); none when the replay wrote the record's log
    pub first_difference: Option<(usize, String, String)>,
}

/// Plays a recorded game again: the same start (without the timer, which only ends games), the recorded calls in
/// order, then no more calls (a record that stops early ends there, as a player who left).
pub fn replay(rec: &Recorded, db: Arc<CardDb>) -> Result<Replayed, String> {
    let mut start = rec.start.clone();
    start.timer = Timer::default();
    start.record = None;
    let replay_log = Arc::new(std::sync::Mutex::new(Vec::new()));
    start.log_to = Some(replay_log.clone());
    let game = HostedGame::start(start, db)?;
    let (calls, events, thread) = game.split();
    for c in &rec.calls {
        let _ = calls.send(c.clone());
    }
    drop(calls);
    // nobody reads the client's events: drain them so the host never waits on them
    let drain = std::thread::spawn(move || while events.recv_timeout(Duration::from_secs(600)).is_ok() {});
    let ended = thread.join().map_err(|_| "the host's thread panicked".to_string())?;
    let _ = drain.join();
    let log = replay_log.lock().unwrap().clone();
    let first_difference = rec
        .log
        .iter()
        .zip(log.iter())
        .enumerate()
        .find(|(_, (a, b))| a.1 != b.1)
        .map(|(i, (a, b))| (i, a.1.clone(), b.1.clone()))
        .or_else(|| match rec.log.len().cmp(&log.len()) {
            std::cmp::Ordering::Equal => None,
            std::cmp::Ordering::Less => Some((rec.log.len(), String::new(), log[rec.log.len()].1.clone())),
            std::cmp::Ordering::Greater => Some((log.len(), rec.log[log.len()].1.clone(), String::new())),
        });
    Ok(Replayed { log, ended, first_difference })
}
