//! A hosted game's record (docs/rust/GameHost.md): its start, every call the human's client sent in the order the host
//! took them, the game log, and the end, one JSON object a line. The engine is deterministic and the AI's search is
//! bounded by answers, not time, so the start and the calls play the same game again ([`replay`]); the client's ids
//! are made from the game's id and the engine's objects, so the recorded calls name the same objects.
//!
//! ```text
//! {"start": {"gameId", "seed", "seats": [{"playerId", "name", "human", "deck", "userData"}], "timer", "chooser", "rules"}}
//! {"call": {"uuid": "…"}, "turn": 3, "ref": "H0.4"}
//! {"log": "<font …>A</font> casts Fatal Push targeting …", "turn": 3}
//! {"end": {"winner": 0, "turns": 9, "reason": "over"}}
//! ```

use crate::game::{Call, Ended, HostedGame, Prelude, SeatStart, Start, TableRules};
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

    pub fn call(&mut self, seat: usize, call: &Call, turn: u32, reference: Option<&str>) {
        let mut v = json!({"call": call_json(call), "turn": turn, "seat": seat});
        if let Some(r) = reference {
            v["ref"] = json!(r);
        }
        self.line(v);
    }

    pub fn end(&mut self, ended: &Ended) {
        self.line(json!({"end": {"winner": ended.winner, "turns": ended.turns, "reason": ended.reason}}));
    }
}

/// A start as the record (and a game's line) writes it
pub fn start_json(s: &Start) -> Value {
    let mut v = json!({
        "gameId": s.game_id,
        "seed": s.seed,
        "seats": s.seats.iter().map(|p| json!({
            "playerId": p.player_id, "name": p.name, "human": p.human, "deck": p.deck, "printings": p.printings, "userData": p.user_data,
        })).collect::<Vec<_>>(),
        "timer": {"prioritySecs": s.timer.priority_secs, "bufferSecs": s.timer.buffer_secs, "left": s.timer.left},
        "chooser": s.chooser,
        "rules": s.rules.json(),
        // what the AI seats played, for `host-results` (the replay doesn't read it)
        "ai": s.rules.ai_config().map(|c| playmat_ai::entrants::label(&c)).unwrap_or_else(|| "quick".into()),
    });
    if let Some(p) = &s.prelude {
        v["prelude"] = prelude_json(p);
    }
    v
}

/// A prelude (a game's line to a turn's start), as `HOST_TURN` sends it and `HOST_GAME` brings it back
pub fn prelude_json(p: &Prelude) -> Value {
    let calls: Vec<Value> = p.calls.iter().map(|(s, c)| json!({"seat": s, "call": call_json(c)})).collect();
    json!({"start": start_json(&p.start), "calls": calls, "turn": p.turn})
}

/// A line read back (`HOST_GAME`'s `prelude`, `HOST_TURN`'s `line`)
pub fn prelude_of(v: &Value) -> Option<Prelude> {
    let start = start_of(&v["start"])?;
    let mut calls = Vec::new();
    for c in v["calls"].as_array()? {
        calls.push((c["seat"].as_u64()? as usize, call_of(&c["call"])?));
    }
    Some(Prelude { start: Box::new(start), calls, turn: v["turn"].as_u64()? as u32 })
}

pub fn start_of(v: &Value) -> Option<Start> {
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
        rules: TableRules::of(&v["rules"]),
        record: None,
        replay: None,
        log_to: None,
        drop_stale: false,
        prelude: match v.get("prelude").filter(|p| !p.is_null()) {
            Some(p) => Some(prelude_of(p)?),
            None => None,
        },
        lines: false,
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
        Call::Advise(skill, _) => json!({"advise": skill}),
        Call::Practice(tool, card, amount, _) => json!({"practice": tool, "card": card, "amount": amount}),
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
    } else if let Some(s) = o.get("advise") {
        let (tx, _) = std::sync::mpsc::channel();
        Call::Advise(u32::try_from(s.as_u64()?).ok()?, tx)
    } else {
        Call::Practice(
            o.get("practice")?.as_str()?.to_string(),
            o.get("card").and_then(Value::as_str).map(str::to_string),
            o.get("amount").and_then(Value::as_i64).unwrap_or(1) as i32,
            None,
        )
    })
}

/// A record read back
pub struct Recorded {
    pub start: Start,
    /// by seat (a record from before games between people names none: the human's)
    pub calls: Vec<(usize, Call)>,
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
            let call = call_of(c).ok_or(format!("line {}: an unknown call", i + 1))?;
            if !matches!(call, Call::Advise(..)) {
                calls.push((v["seat"].as_u64().map(|s| s as usize), call));
            }
        } else if let Some(l) = v.get("log").and_then(Value::as_str) {
            log.push((v["turn"].as_u64().unwrap_or(0) as u32, l.to_string()));
        } else if let Some(e) = v.get("end") {
            end = Some(e.clone());
        }
    }
    let start: Start = start.ok_or("no start line")?;
    let human = start.seats.iter().position(|s| s.human).unwrap_or(0);
    let calls = calls.into_iter().map(|(s, c)| (s.unwrap_or(human), c)).collect();
    Ok(Recorded { start, calls, log, end })
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
    for (seat, c) in &rec.calls {
        let _ = calls.send((*seat, c.clone()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn call_round_trips_every_variant() {
        let (tx, _rx) = channel();
        let cases = [
            Call::Join,
            Call::Uuid(Some("obj-1".into())),
            Call::Uuid(None),
            Call::Bool(true),
            Call::Int(7),
            Call::Str("hello".into()),
            Call::ManaType("GREEN".into()),
            Call::Action("PASS".into(), Some(json!({"x": 1}))),
            Call::Resync,
            Call::Quit,
            Call::Advise(5, tx),
            Call::Practice("DRAW".into(), Some("Forest".into()), 2, None),
        ];
        for c in &cases {
            let v = call_json(c);
            let back = call_of(&v).unwrap_or_else(|| panic!("call_of failed for {v}"));
            match (c, back) {
                (Call::Join, Call::Join) | (Call::Resync, Call::Resync) | (Call::Quit, Call::Quit) => {}
                (Call::Uuid(a), Call::Uuid(b)) => assert_eq!(*a, b),
                (Call::Bool(a), Call::Bool(b)) => assert_eq!(*a, b),
                (Call::Int(a), Call::Int(b)) => assert_eq!(*a, b),
                (Call::Str(a), Call::Str(b)) => assert_eq!(*a, b),
                (Call::ManaType(a), Call::ManaType(b)) => assert_eq!(*a, b),
                (Call::Action(a, da), Call::Action(b, db)) => {
                    assert_eq!(*a, b);
                    assert_eq!(*da, db);
                }
                (Call::Advise(a, _), Call::Advise(b, _)) => assert_eq!(*a, b),
                (Call::Practice(ta, ca, na, _), Call::Practice(tb, cb, nb, _)) => {
                    assert_eq!(*ta, tb);
                    assert_eq!(*ca, cb);
                    assert_eq!(*na, nb);
                }
                (expected, got) => panic!("mismatch: expected {expected:?}, got {got:?}"),
            }
        }
    }

    #[test]
    fn read_handles_and_skips_recorded_advise_calls() {
        let dir = std::env::temp_dir().join(format!("playmat-record-test-{}", std::process::id()));
        let path = dir.join("advise.jsonl");
        let start = Start {
            game_id: "g-advise".into(),
            seed: 1,
            seats: vec![
                SeatStart {
                    player_id: "p0".into(),
                    name: "Alice".into(),
                    human: true,
                    deck: vec!["Mountain".into()],
                    printings: vec![],
                    user_data: None,
                },
                SeatStart {
                    player_id: "p1".into(),
                    name: "Bot".into(),
                    human: false,
                    deck: vec!["Mountain".into()],
                    printings: vec![],
                    user_data: None,
                },
            ],
            rules: TableRules::default(),
            timer: Timer::default(),
            chooser: None,
            record: None,
            replay: None,
            log_to: None,
            drop_stale: false,
            prelude: None,
            lines: false,
        };
        let (tx, _rx) = channel();
        {
            let mut rec = Recorder::create(&path, &start).unwrap();
            rec.call(0, &Call::Join, 1, None);
            rec.call(0, &Call::Advise(4, tx), 1, None);
            rec.call(0, &Call::Bool(true), 1, None);
        }
        let recorded = read(&path).expect("record with advise call should be readable");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(recorded.calls.len(), 2);
        assert!(matches!(recorded.calls[0], (0, Call::Join)));
        assert!(matches!(recorded.calls[1], (0, Call::Bool(true))));
    }
}
