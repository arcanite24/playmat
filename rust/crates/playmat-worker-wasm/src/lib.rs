//! Mana@Home browser and Android WebView worker runtime (S6, `docs/swarm/S6.md`).
//!
//! Compiles to `wasm32-unknown-unknown` with `--features strict` so browser Web Workers and Capacitor Android
//! WebViews join the exact same `strict-1` determinism class as native CPU workers, playing one seeded deal per
//! single-threaded Web Worker with zero external JS-glue crate dependencies.

use playmat_ai::answerlog::AnswerLog;
use playmat_ai::duel::Deck;
use playmat_ai::entrants::entrant;
use playmat_ai::play::{self, Pairing, Recorder, Setup};
use playmat_swarm_proto as proto;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize)]
pub struct DeckInput {
    pub name: String,
    pub cards: Vec<(u32, String)>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlayDealRequest {
    pub deal: usize,
    pub decks: Vec<DeckInput>,
    pub learner: String,
    pub opponent: String,
    #[serde(default = "default_turn_cap")]
    pub turn_cap: u32,
    #[serde(default)]
    pub mulligans: bool,
    #[serde(default)]
    pub keep: u32,
}

fn default_turn_cap() -> u32 {
    30
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayDealReply {
    pub deal: usize,
    pub class: String,
    pub winner: Option<usize>,
    pub turns: u32,
    pub answers: usize,
    pub recorded: usize,
    pub log_sha256: String,
    pub steps_sha256: String,
    pub log: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReplayDealRequest {
    pub deal: usize,
    pub decks: Vec<DeckInput>,
    pub log: String,
    #[serde(default = "default_turn_cap")]
    pub turn_cap: u32,
    #[serde(default)]
    pub mulligans: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayDealReply {
    pub deal: usize,
    pub winner: Option<usize>,
    pub turns: u32,
    pub answers: usize,
    pub log_sha256: String,
}

fn build_decks(inputs: &[DeckInput]) -> Result<Vec<Deck>, String> {
    if inputs.len() < 2 {
        return Err("at least two decks are required".into());
    }
    Ok(inputs
        .iter()
        .map(|d| {
            let mut cards = Vec::new();
            for (count, name) in &d.cards {
                for _ in 0..*count {
                    cards.push(name.clone());
                }
            }
            Deck { name: d.name.clone(), cards }
        })
        .collect())
}

/// Returns this build's determinism class (`strict-1` under the default `strict` feature).
pub fn determinism_class() -> String {
    proto::determinism_class(playmat_ai::STRICT)
}

/// Runs one seeded deal in strict mode and returns its JSON-serialized `PlayDealReply`.
pub fn play_deal(req: PlayDealRequest) -> Result<PlayDealReply, String> {
    let decks = build_decks(&req.decks)?;
    let db = playmat_engine::cards();
    let setup = Setup { db: &db, decks: &decks, turns: req.turn_cap, mulligans: req.mulligans };
    let pairing = play::pairing(req.deal, decks.len(), &[1]);
    let learner = entrant(&format!("{}+threads=1+par=1", req.learner))?;
    let opp =
        if req.opponent == "self" { learner.clone() } else { entrant(&format!("{}+threads=1+par=1", req.opponent))? };
    let mut configs = [opp.config.clone(), opp.config.clone()];
    configs[pairing.me] = learner.config.clone();
    let both = req.opponent == "self";
    let record = [req.keep > 0 && (both || pairing.me == 0), req.keep > 0 && (both || pairing.me == 1)];
    let rec = Recorder { keep: req.keep, fast: None, fast_iters: None };
    let played = play::play_game(&setup, req.deal, pairing, configs, record, rec)?;
    Ok(PlayDealReply {
        deal: req.deal,
        class: determinism_class(),
        winner: played.winner,
        turns: played.turns,
        answers: played.log.entries.len(),
        recorded: played.steps.len(),
        log_sha256: played.log.hash(),
        steps_sha256: play::steps_hash(&played.steps),
        log: played.log.encode(),
    })
}

/// Replays one seeded deal from its answer log alone (no search) to verify legality and outcome.
pub fn replay_deal(req: ReplayDealRequest) -> Result<ReplayDealReply, String> {
    let decks = build_decks(&req.decks)?;
    let db = playmat_engine::cards();
    let setup = Setup { db: &db, decks: &decks, turns: req.turn_cap, mulligans: req.mulligans };
    let pairing: Pairing = play::pairing(req.deal, decks.len(), &[1]);
    let log = AnswerLog::decode(&req.log)?;
    let log_sha256 = log.hash();
    let answers = log.entries.len();
    let out = play::replay_game(&setup, req.deal, pairing, &log)?;
    Ok(ReplayDealReply { deal: req.deal, winner: out.winner, turns: out.turns, answers, log_sha256 })
}

pub fn play_deal_json(input: &str) -> String {
    match serde_json::from_str::<PlayDealRequest>(input).map_err(|e| e.to_string()).and_then(play_deal) {
        Ok(rep) => serde_json::to_string(&rep).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
        Err(e) => serde_json::to_string(&BTreeMap::from([("error", e)])).unwrap(),
    }
}

pub fn replay_deal_json(input: &str) -> String {
    match serde_json::from_str::<ReplayDealRequest>(input).map_err(|e| e.to_string()).and_then(replay_deal) {
        Ok(rep) => serde_json::to_string(&rep).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
        Err(e) => serde_json::to_string(&BTreeMap::from([("error", e)])).unwrap(),
    }
}

// ---- C/WASM ABI exports for zero-glue Web Workers (`WebAssembly.instantiate`)

#[no_mangle]
pub extern "C" fn wasm_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Frees a buffer previously returned by `wasm_alloc` or `wasm_*_json`.
///
/// # Safety
/// `ptr` must have been allocated by `wasm_alloc` / `pack_string` with capacity `len`.
#[no_mangle]
pub unsafe extern "C" fn wasm_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

fn pack_string(s: String) -> u64 {
    let mut bytes = s.into_bytes().into_boxed_slice();
    let ptr = bytes.as_mut_ptr() as u32;
    let len = bytes.len() as u32;
    std::mem::forget(bytes);
    ((ptr as u64) << 32) | (len as u64)
}

#[no_mangle]
pub extern "C" fn wasm_determinism_class() -> u64 {
    pack_string(determinism_class())
}

/// Runs `play_deal_json` on UTF-8 JSON bytes at `(ptr, len)` and returns packed `(out_ptr << 32) | out_len`.
///
/// # Safety
/// `(ptr, len)` must point to valid UTF-8 bytes inside WASM linear memory.
#[no_mangle]
pub unsafe extern "C" fn wasm_play_deal_json(ptr: *const u8, len: usize) -> u64 {
    let slice = std::slice::from_raw_parts(ptr, len);
    let input = std::str::from_utf8(slice).unwrap_or("");
    pack_string(play_deal_json(input))
}

/// Runs `replay_deal_json` on UTF-8 JSON bytes at `(ptr, len)` and returns packed `(out_ptr << 32) | out_len`.
///
/// # Safety
/// `(ptr, len)` must point to valid UTF-8 bytes inside WASM linear memory.
#[no_mangle]
pub unsafe extern "C" fn wasm_replay_deal_json(ptr: *const u8, len: usize) -> u64 {
    let slice = std::slice::from_raw_parts(ptr, len);
    let input = std::str::from_utf8(slice).unwrap_or("");
    pack_string(replay_deal_json(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_worker_plays_and_replays_strict_deal() {
        assert_eq!(determinism_class(), "strict-1");
        let deck_a = DeckInput {
            name: "red-test".into(),
            cards: vec![(24, "Mountain".into()), (18, "Lightning Bolt".into()), (18, "Shock".into())],
        };
        let deck_b = DeckInput {
            name: "green-test".into(),
            cards: vec![(24, "Forest".into()), (18, "Llanowar Elves".into()), (18, "Giant Growth".into())],
        };
        let played = play_deal(PlayDealRequest {
            deal: 42,
            decks: vec![deck_a.clone(), deck_b.clone()],
            learner: "cfg:policy=quick".into(),
            opponent: "self".into(),
            turn_cap: 12,
            mulligans: false,
            keep: 0,
        })
        .unwrap();
        assert!(played.answers > 0);
        let replayed = replay_deal(ReplayDealRequest {
            deal: 42,
            decks: vec![deck_a, deck_b],
            log: played.log,
            turn_cap: 12,
            mulligans: false,
        })
        .unwrap();
        assert_eq!(replayed.winner, played.winner);
        assert_eq!(replayed.turns, played.turns);
        assert_eq!(replayed.log_sha256, played.log_sha256);
    }
}
