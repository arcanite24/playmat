//! Mana@Home browser and Android WebView worker runtime (S6, `docs/swarm/S6.md`).
//!
//! Compiles to `wasm32-unknown-unknown` with `--features strict` so browser Web Workers and Android workers join
//! the exact same `strict-1` determinism class as native CPU workers, playing seeded units and producing rich
//! human-readable Live Watch replay traces with zero external JS-glue crate dependencies.

use playmat_ai::answerlog::AnswerLog;
use playmat_ai::duel::Deck;
use playmat_ai::entrants::entrant;
use playmat_ai::play::{self, Pairing, Recorder, ReplayedTrace, Setup};
use playmat_ai::selfplay::Shard;
use playmat_swarm_proto::{self as proto, GameSummary, ResultManifest, UnitKind, WorkUnit, PROTOCOL};
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

#[derive(Clone, Debug, Deserialize)]
pub struct RunUnitRequest {
    pub unit: WorkUnit,
    pub decks: Vec<DeckInput>,
    pub worker: String,
    pub build: String,
    #[serde(default = "default_turn_cap")]
    pub turn_cap: u32,
    #[serde(default = "default_mulligans")]
    pub mulligans: bool,
}

fn default_mulligans() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub base64: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunUnitReply {
    pub manifest: ResultManifest,
    pub files: Vec<EncodedFile>,
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

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Returns this build's determinism class (`strict-1` under the default `strict` feature).
pub fn determinism_class() -> String {
    proto::determinism_class(playmat_ai::STRICT)
}

/// Verifies SHA-256 of raw `.pmnet` bytes and registers the network in memory under `@<sha256>`.
pub fn register_net(expected_sha: &str, bytes: &[u8]) -> Result<(), String> {
    let actual = proto::sha256_hex(bytes);
    if actual != expected_sha {
        return Err(format!("net sha256 mismatch: expected {expected_sha}, got {actual}"));
    }
    playmat_ai::net::register_bytes(&format!("@{expected_sha}"), bytes).map(|_| ())
}

/// Runs one seeded deal in strict mode and returns its `PlayDealReply`.
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

/// Runs a full coordinator `WorkUnit` in memory (strict single-threaded) and returns its manifest + upload files.
pub fn run_unit_mem(req: RunUnitRequest) -> Result<RunUnitReply, String> {
    let unit = &req.unit;
    if unit.protocol != PROTOCOL {
        return Err(format!("protocol {} != {PROTOCOL}", unit.protocol));
    }
    if unit.kind == UnitKind::SelfplayGpu {
        return Err("WASM worker does not execute GPU units".into());
    }
    let decks = build_decks(&req.decks)?;
    let db = playmat_engine::cards();
    let setup = Setup { db: &db, decks: &decks, turns: req.turn_cap, mulligans: req.mulligans };
    let learner = entrant(&format!("{}+threads=1+par=1", unit.learner))?;
    let mut opps = Vec::with_capacity(unit.opponents.len());
    let mut weights = Vec::with_capacity(unit.opponents.len());
    for o in &unit.opponents {
        let e = if o.config == "self" { learner.clone() } else { entrant(&format!("{}+threads=1+par=1", o.config))? };
        opps.push((o.config == "self", e));
        weights.push(o.weight);
    }
    if opps.is_empty() {
        return Err("unit has no opponents".into());
    }
    let (fast, fast_iters) = match &unit.fast {
        Some(f) => play::parse_fast(f)?,
        None => (None, None),
    };
    let rec = Recorder { keep: unit.keep, fast, fast_iters };
    let record_unit = unit.kind == UnitKind::Selfplay && unit.keep > 0;
    let mut shard = if record_unit { Some(Shard::<Vec<u8>>::create_mem()) } else { None };
    let mut answers_lines = Vec::with_capacity(unit.games as usize);
    let mut games = Vec::with_capacity(unit.games as usize);
    let mut failed = Vec::new();

    for i in 0..unit.games {
        let deal = (unit.seed + i as u64) as usize;
        let pairing = play::pairing(deal, decks.len(), &weights);
        let (is_self, opp) = &opps[pairing.opponent];
        let mut configs = [opp.config.clone(), opp.config.clone()];
        configs[pairing.me] = learner.config.clone();
        let record = [record_unit && (*is_self || pairing.me == 0), record_unit && (*is_self || pairing.me == 1)];
        match play::play_game(&setup, deal, pairing, configs, record, rec) {
            Ok(played) => {
                let log_sha256 = played.log.hash();
                let steps_sha256 = play::steps_hash(&played.steps);
                let result = match played.winner {
                    Some(0) => [1.0, 0.0],
                    Some(1) => [0.0, 1.0],
                    _ => [0.5, 0.5],
                };
                if let Some(s) = shard.as_mut() {
                    let info = serde_json::json!({
                        "deal": deal,
                        "decks": [decks[pairing.decks[0]].name, decks[pairing.decks[1]].name],
                        "first": pairing.first,
                        "turns": played.turns,
                    });
                    s.game(deal, pairing.opponent as i32, &played.steps, result, info).map_err(|e| e.to_string())?;
                }
                let encoded_log = played.log.encode();
                answers_lines.push(
                    serde_json::json!({
                        "deal": deal,
                        "log_sha256": log_sha256,
                        "log": encoded_log,
                    })
                    .to_string(),
                );
                games.push(GameSummary {
                    deal: deal as u64,
                    opponent: pairing.opponent as u32,
                    learner: pairing.me as u8,
                    winner: played.winner.map(|w| w as u8),
                    turns: played.turns,
                    answers: played.log.entries.len() as u32,
                    recorded: played.steps.len() as u32,
                    log_sha256,
                    steps_sha256,
                    policy_entropy: None,
                });
            }
            Err(_) => failed.push(deal as u64),
        }
    }

    let mut raw_files: Vec<(String, Vec<u8>)> = Vec::new();
    let answers_bytes = format!("{}\n", answers_lines.join("\n")).into_bytes();
    raw_files.push(("answers.jsonl".into(), answers_bytes));
    if let Some(s) = shard {
        raw_files.extend(s.into_files("w0000"));
    }

    let mut file_hashes = BTreeMap::new();
    let mut encoded_files = Vec::with_capacity(raw_files.len());
    for (path, bytes) in raw_files {
        let sha256 = proto::sha256_hex(&bytes);
        let size = bytes.len() as u64;
        file_hashes.insert(path.clone(), sha256.clone());
        encoded_files.push(EncodedFile { path, size, sha256, base64: encode_base64(&bytes) });
    }

    let manifest = ResultManifest {
        protocol: PROTOCOL,
        unit: unit.id.clone(),
        worker: req.worker,
        build: req.build,
        class: determinism_class(),
        nets: unit.nets.iter().map(|n| n.sha256.clone()).collect(),
        games,
        failed,
        files: file_hashes,
        elapsed_ms: 0,
    };
    Ok(RunUnitReply { manifest, files: encoded_files })
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

/// Replays one seeded deal from its answer log and returns a human-readable turn-by-turn board trace.
pub fn replay_trace(req: ReplayDealRequest) -> Result<ReplayedTrace, String> {
    let decks = build_decks(&req.decks)?;
    let db = playmat_engine::cards();
    let setup = Setup { db: &db, decks: &decks, turns: req.turn_cap, mulligans: req.mulligans };
    let pairing: Pairing = play::pairing(req.deal, decks.len(), &[1]);
    let log = AnswerLog::decode(&req.log)?;
    play::replay_trace(&setup, req.deal, pairing, &log)
}

pub fn play_deal_json(input: &str) -> String {
    match serde_json::from_str::<PlayDealRequest>(input).map_err(|e| e.to_string()).and_then(play_deal) {
        Ok(rep) => serde_json::to_string(&rep).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
        Err(e) => serde_json::to_string(&BTreeMap::from([("error", e)])).unwrap(),
    }
}

pub fn run_unit_json(input: &str) -> String {
    match serde_json::from_str::<RunUnitRequest>(input).map_err(|e| e.to_string()).and_then(run_unit_mem) {
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

pub fn replay_trace_json(input: &str) -> String {
    match serde_json::from_str::<ReplayDealRequest>(input).map_err(|e| e.to_string()).and_then(replay_trace) {
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

/// Verifies and registers raw `.pmnet` bytes under `@<sha256>` in memory.
///
/// # Safety
/// Pointer/length pairs must refer to valid memory inside WASM linear memory.
#[no_mangle]
pub unsafe extern "C" fn wasm_register_net(
    sha_ptr: *const u8,
    sha_len: usize,
    bytes_ptr: *const u8,
    bytes_len: usize,
) -> u64 {
    let sha = std::str::from_utf8(std::slice::from_raw_parts(sha_ptr, sha_len)).unwrap_or("");
    let bytes = std::slice::from_raw_parts(bytes_ptr, bytes_len);
    match register_net(sha, bytes) {
        Ok(()) => pack_string("{\"ok\":true}".into()),
        Err(e) => pack_string(serde_json::to_string(&BTreeMap::from([("error", e)])).unwrap()),
    }
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

/// Runs `run_unit_json` on UTF-8 JSON bytes at `(ptr, len)` and returns packed `(out_ptr << 32) | out_len`.
///
/// # Safety
/// `(ptr, len)` must point to valid UTF-8 bytes inside WASM linear memory.
#[no_mangle]
pub unsafe extern "C" fn wasm_run_unit_json(ptr: *const u8, len: usize) -> u64 {
    let slice = std::slice::from_raw_parts(ptr, len);
    let input = std::str::from_utf8(slice).unwrap_or("");
    pack_string(run_unit_json(input))
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

/// Runs `replay_trace_json` on UTF-8 JSON bytes at `(ptr, len)` and returns packed `(out_ptr << 32) | out_len`.
///
/// # Safety
/// `(ptr, len)` must point to valid UTF-8 bytes inside WASM linear memory.
#[no_mangle]
pub unsafe extern "C" fn wasm_replay_trace_json(ptr: *const u8, len: usize) -> u64 {
    let slice = std::slice::from_raw_parts(ptr, len);
    let input = std::str::from_utf8(slice).unwrap_or("");
    pack_string(replay_trace_json(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_worker_plays_replays_and_traces_strict_deal() {
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
            decks: vec![deck_a.clone(), deck_b.clone()],
            log: played.log.clone(),
            turn_cap: 12,
            mulligans: false,
        })
        .unwrap();
        assert_eq!(replayed.winner, played.winner);
        assert_eq!(replayed.turns, played.turns);
        assert_eq!(replayed.log_sha256, played.log_sha256);

        let trace = replay_trace(ReplayDealRequest {
            deal: 42,
            decks: vec![deck_a, deck_b],
            log: played.log,
            turn_cap: 12,
            mulligans: false,
        })
        .unwrap();
        assert_eq!(trace.winner, played.winner);
        assert_eq!(trace.frames.len(), played.answers);
        assert!(!trace.frames[0].action.is_empty());
    }
}
