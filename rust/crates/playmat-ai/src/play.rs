//! One recorded game, and its replay: what the `selfplay` bin and the Mana@Home worker share (docs/swarm/Design.md).
//!
//! A deal fixes everything but the answers: the decks (the suite's ordered pairs, in turn), who plays first, the
//! game's seed, the learner's seat and the opponent. `play_game` plays it with searches, recording training steps and
//! the full answer log; `replay_game` plays the same deal again from the log alone, with no search, so anyone can
//! check a game in a few milliseconds.

use crate::answerlog::{AnswerLog, Entry, Logged, Replayer};
use crate::decision::PolicyKind;
use crate::duel::{new_duel, Deck};
use crate::search::{Config, Table};
use crate::selfplay::{mix, Recording, Sink, Step};
use crate::{eval, mulligan};
use playmat_engine::rules;
use playmat_engine::state::{Game, Seat};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

/// What every game of a run shares
pub struct Setup<'a> {
    pub db: &'a playmat_engine::card::CardDb,
    pub decks: &'a [Deck],
    pub turns: u32,
    pub mulligans: bool,
}

/// What a deal number decides
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pairing {
    /// seat 0's deck and seat 1's
    pub decks: [usize; 2],
    /// who plays first
    pub first: Seat,
    /// the learner's seat
    pub me: Seat,
    /// the opponent, by its index in the run's weights
    pub opponent: usize,
}

pub fn pairing(deal: usize, n_decks: usize, weights: &[u64]) -> Pairing {
    let pair = deal / 2;
    let swap = deal & 1;
    let mut r = (pair as u64) ^ 0xa076_1d64_78bd_642f;
    let (a, b) = if n_decks <= 1 {
        (0, 0)
    } else {
        let a = (mix(&mut r) as usize) % n_decks;
        let b = (a + 1 + (mix(&mut r) as usize) % (n_decks - 1)) % n_decks;
        (a, b)
    };
    let total: u64 = weights.iter().sum::<u64>().max(1);
    let mut pick = mix(&mut r) % total;
    let opponent = weights
        .iter()
        .position(|w| {
            if pick < *w {
                true
            } else {
                pick -= w;
                false
            }
        })
        .unwrap_or(0);
    let base_bits = mix(&mut r);
    let first = (base_bits & 1) as usize;
    let base_me = ((base_bits >> 1) & 1) as usize;
    let me = base_me ^ swap;
    Pairing { decks: [a, b], first, me, opponent }
}

/// The game's seed, from its deal
pub fn seed(deal: usize) -> i64 {
    deal as i64 + 7_000_000
}

/// How the recorded seats answer the decisions they don't record: `--fast net`, `--fast quick`, `--fast ismcts:12`
pub fn parse_fast(v: &str) -> Result<(Option<PolicyKind>, Option<u32>), String> {
    if let Some(n) = v.strip_prefix("ismcts:") {
        return Ok((None, Some(n.parse().map_err(|e| format!("fast {v}: {e}"))?)));
    }
    Ok((Some(v.parse().map_err(|e: String| format!("fast {v}: {e}"))?), None))
}

/// The seat that won, `None` for a draw or a game cut at the turn cap
pub fn winner(game: &Game) -> Option<Seat> {
    if game.player(0).lost || eval::terminal_outcome(game, 1) == Some(true) {
        Some(1)
    } else if game.player(1).lost || eval::terminal_outcome(game, 0) == Some(true) {
        Some(0)
    } else {
        None
    }
}

/// How the recorded seats record
#[derive(Clone, Copy, Debug)]
pub struct Recorder {
    pub keep: u32,
    pub fast: Option<PolicyKind>,
    pub fast_iters: Option<u32>,
}

pub struct Played {
    pub winner: Option<Seat>,
    pub turns: u32,
    pub steps: Vec<Step>,
    pub log: AnswerLog,
}

/// Plays a deal: `configs` per seat (`None` = the rule AI), `record` the seats whose decisions become training steps.
/// Fails if the deal can't be set up or the table stopped the game early (its answer guard).
pub fn play_game(
    setup: &Setup,
    deal: usize,
    p: Pairing,
    configs: [Option<Config>; 2],
    record: [bool; 2],
    rec: Recorder,
) -> Result<Played, String> {
    let decks = [&setup.decks[p.decks[0]], &setup.decks[p.decks[1]]];
    let mut game = new_duel(setup.db, decks, seed(deal), p.first)?;
    let log = RefCell::new(AnswerLog::default());
    if setup.mulligans {
        mulligan::london(
            &mut game,
            p.first,
            0,
            &mut |g, seat, taken| {
                let m = mulligan::wants(g, seat, taken, configs[seat].as_ref());
                log.borrow_mut().entries.push(Entry::Mulligan(seat, m));
                m
            },
            &mut |g, seat| {
                let card = mulligan::bottom(g, seat, configs[seat].as_ref());
                log.borrow_mut().entries.push(Entry::Bottom(seat, card));
                card
            },
        );
    }
    let mut log = log.into_inner();
    let sink: Sink = Arc::new(Mutex::new(Vec::new()));
    let mut table = Table::new(configs.to_vec());
    for seat in 0..2 {
        if let (true, Some(c)) = (record[seat], configs[seat].as_ref()) {
            let r = Recording::new(c.policy, seat, Arc::clone(&sink), rec.keep, (deal as u64) << 1 | seat as u64)
                .with_fast(rec.fast, seat)
                .with_fast_iters(rec.fast_iters);
            table.set_policy(seat, Box::new(r));
        }
    }
    rules::play_from_priority(&mut game, &mut Logged { inner: &mut table, log: &mut log }, setup.turns);
    if table.stopped_early() {
        return Err(format!("deal {deal}: the table stopped the game"));
    }
    let steps = std::mem::take(&mut *sink.lock().unwrap());
    Ok(Played { winner: winner(&game), turns: game.turn, steps, log })
}

/// sha256 of a game's training steps, in hex: every observation, target and search value as recorded, so two runs
/// of a game can be checked for the same training data and not only the same answers (floats print exactly: Rust's
/// shortest round-trip form)
pub fn steps_hash(steps: &[Step]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(format!("{steps:?}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// How a replayed game ended
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replayed {
    pub winner: Option<Seat>,
    pub turns: u32,
}

/// One human-readable decision snapshot produced by `replay_trace` (used by Live Watch).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TraceFrame {
    pub index: usize,
    pub turn: u32,
    pub step: String,
    pub active: usize,
    pub seat: usize,
    pub life: [i32; 2],
    pub hands: [usize; 2],
    pub board: [Vec<String>; 2],
    pub stack: Vec<String>,
    pub action: String,
}

/// Full human-readable replay trace of a game (`replay_trace`).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReplayedTrace {
    pub decks: [String; 2],
    pub first: usize,
    pub winner: Option<Seat>,
    pub turns: u32,
    pub frames: Vec<TraceFrame>,
}

fn snapshot_board(game: &Game) -> [Vec<String>; 2] {
    let mut b = [Vec::new(), Vec::new()];
    for &id in game.battlefield.iter() {
        let o = game.obj(id);
        let label = if o.tapped { format!("{} (T)", o.name()) } else { o.name().to_string() };
        if o.controller < 2 {
            b[o.controller].push(label);
        }
    }
    b
}

fn snapshot_stack(game: &Game) -> Vec<String> {
    game.stack.iter().map(|s| game.obj(s.card).name().to_string()).collect()
}

fn describe_answer(game: &Game, seat: Seat, a: &playmat_engine::decide::Answer) -> String {
    use playmat_engine::decide::Answer;
    match a {
        Answer::Bool(true) => format!("P{seat}: Yes / Continue"),
        Answer::Bool(false) => format!("P{seat}: Pass / No"),
        Answer::Int(n) => format!("P{seat}: Choose {n}"),
        Answer::Text(t) => format!("P{seat}: Choose {t}"),
        Answer::ManaType(m) => format!("P{seat}: Add {m} mana"),
        Answer::Player(p) => format!("P{seat}: Target Player {p}"),
        Answer::Obj(id) => format!("P{seat}: Play / Choose {}", game.obj(*id).name()),
        Answer::Ability(id, k) => format!("P{seat}: Activate {} (#{k})", game.obj(*id).name()),
        Answer::Trigger(k) => format!("P{seat}: Order trigger #{k}"),
        Answer::Mode(k) => format!("P{seat}: Choose mode #{k}"),
        Answer::ModeDone => format!("P{seat}: Done choosing modes"),
        Answer::ModeCancel => format!("P{seat}: Cancel"),
        Answer::Unknown => format!("P{seat}: Unknown"),
        Answer::Stop => format!("P{seat}: Stop"),
    }
}

struct TracingReplayer<'a> {
    inner: Replayer<'a>,
    frames: Vec<TraceFrame>,
}

impl playmat_engine::decide::Decider for TracingReplayer<'_> {
    fn answer(
        &mut self,
        game: &Game,
        seat: Seat,
        query: &playmat_engine::decide::Query,
    ) -> playmat_engine::decide::Answer {
        let idx = self.inner.pos;
        let a = self.inner.answer(game, seat, query);
        if self.inner.problem.is_none() {
            self.frames.push(TraceFrame {
                index: idx,
                turn: game.turn,
                step: game.step.map(|s| format!("{s:?}")).unwrap_or_else(|| "Setup".into()),
                active: game.active.unwrap_or(0),
                seat,
                life: [game.player(0).life, game.player(1).life],
                hands: [game.player(0).hand.len(), game.player(1).hand.len()],
                board: snapshot_board(game),
                stack: snapshot_stack(game),
                action: describe_answer(game, seat, &a),
            });
        }
        a
    }

    fn stopped(&self) -> bool {
        self.inner.stopped()
    }
}

/// Replays a deal from its answer log and emits a human-readable turn-by-turn board trace.
pub fn replay_trace(setup: &Setup, deal: usize, p: Pairing, log: &AnswerLog) -> Result<ReplayedTrace, String> {
    let decks = [&setup.decks[p.decks[0]], &setup.decks[p.decks[1]]];
    let mut game = new_duel(setup.db, decks, seed(deal), p.first)?;
    let replayer = RefCell::new(TracingReplayer { inner: Replayer::new(log), frames: Vec::new() });
    if setup.mulligans {
        mulligan::london(
            &mut game,
            p.first,
            0,
            &mut |g, seat, _| {
                let mut tr = replayer.borrow_mut();
                let idx = tr.inner.pos;
                let m = tr.inner.mulligan(seat);
                if tr.inner.problem.is_none() {
                    tr.frames.push(TraceFrame {
                        index: idx,
                        turn: 0,
                        step: "Mulligan".into(),
                        active: p.first,
                        seat,
                        life: [g.player(0).life, g.player(1).life],
                        hands: [g.player(0).hand.len(), g.player(1).hand.len()],
                        board: snapshot_board(g),
                        stack: vec![],
                        action: if m { format!("P{seat}: Mulligan") } else { format!("P{seat}: Keep hand") },
                    });
                }
                m
            },
            &mut |g, seat| {
                let mut tr = replayer.borrow_mut();
                let idx = tr.inner.pos;
                let id = tr.inner.bottom(g, seat);
                if tr.inner.problem.is_none() {
                    tr.frames.push(TraceFrame {
                        index: idx,
                        turn: 0,
                        step: "Mulligan".into(),
                        active: p.first,
                        seat,
                        life: [g.player(0).life, g.player(1).life],
                        hands: [g.player(0).hand.len(), g.player(1).hand.len()],
                        board: snapshot_board(g),
                        stack: vec![],
                        action: format!("P{seat}: Bottom {}", g.obj(id).name()),
                    });
                }
                id
            },
        );
    }
    let mut tr = replayer.into_inner();
    rules::play_from_priority(&mut game, &mut tr, setup.turns);
    if let Some(why) = tr.inner.problem {
        return Err(format!("deal {deal}: {why}"));
    }
    if !tr.inner.finished() {
        return Err(format!("deal {deal}: the game ended with {} of {} answers used", tr.inner.pos, log.entries.len()));
    }
    Ok(ReplayedTrace {
        decks: [decks[0].name.clone(), decks[1].name.clone()],
        first: p.first,
        winner: winner(&game),
        turns: game.turn,
        frames: tr.frames,
    })
}

/// Plays a deal again from its answer log. Fails if any answer doesn't fit the game or the log isn't used up.
pub fn replay_game(setup: &Setup, deal: usize, p: Pairing, log: &AnswerLog) -> Result<Replayed, String> {
    let decks = [&setup.decks[p.decks[0]], &setup.decks[p.decks[1]]];
    let mut game = new_duel(setup.db, decks, seed(deal), p.first)?;
    let replayer = RefCell::new(Replayer::new(log));
    if setup.mulligans {
        mulligan::london(
            &mut game,
            p.first,
            0,
            &mut |_, seat, _| replayer.borrow_mut().mulligan(seat),
            &mut |g, seat| replayer.borrow_mut().bottom(g, seat),
        );
    }
    let mut replayer = replayer.into_inner();
    rules::play_from_priority(&mut game, &mut replayer, setup.turns);
    if let Some(why) = replayer.problem {
        return Err(format!("deal {deal}: {why}"));
    }
    if !replayer.finished() {
        return Err(format!("deal {deal}: the game ended with {} of {} answers used", replayer.pos, log.entries.len()));
    }
    Ok(Replayed { winner: winner(&game), turns: game.turn })
}

/// The suite's decks, read from the repository (deck files are named from its root)
pub fn load_suite(repo: &str, suite_dir: &str) -> Result<(serde_json::Value, Vec<Deck>), String> {
    let text = std::fs::read_to_string(format!("{suite_dir}/suite.json")).map_err(|e| format!("{suite_dir}: {e}"))?;
    let suite: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("suite.json: {e}"))?;
    let mut decks = Vec::new();
    for d in suite["decks"].as_array().ok_or("suite.json: no decks")? {
        let path = d["file"].as_str().ok_or("suite.json: a deck without a file")?;
        let text = std::fs::read_to_string(format!("{repo}/{path}")).map_err(|e| format!("{path}: {e}"))?;
        let name = std::path::Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path);
        decks.push(Deck::parse(name, &text));
    }
    Ok((suite, decks))
}
