//! One decision at a time: the interface every decider of the AI's seat answers through (docs/rust/Ai.md).
//!
//! The `Table` asks its seat's `DecisionPolicy` each question that has two or more answers worth trying. The policy
//! gets the game, the question, the quick answer, the candidates and a `Root` to play candidates out on copies; it
//! answers with one of the candidates (or anything the question accepts), or `None` to let the quick answer stand.
//! Shipped policies: `Playouts` (the default search), `Mcts`, `Quick` and `Random`. A Jev, LLM or ONNX decider is
//! another implementation, set on a table with `Table::set_policy`.

use crate::eval;
use crate::policy::{Policy, Repeats};
use crate::search::{candidates, Config};
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::rng::JavaRandom;
use playmat_engine::rules;
use playmat_engine::state::{Game, Seat};
use std::str::FromStr;

/// Which `DecisionPolicy` a seat's `Config` builds (`PLAYMAT_AI_POLICY`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolicyKind {
    /// each candidate played once to the end of the turn, the best end of turn kept (`search::Playouts`)
    #[default]
    Playouts,
    /// Monte Carlo tree search at the root, playouts on shuffled copies in parallel threads (`Mcts`)
    Mcts,
    /// the quick answers alone, no search
    Quick,
    /// a random answer among the candidates (Archbench's anchor)
    Random,
}

impl PolicyKind {
    pub fn name(self) -> &'static str {
        match self {
            PolicyKind::Playouts => "playouts",
            PolicyKind::Mcts => "mcts",
            PolicyKind::Quick => "quick",
            PolicyKind::Random => "random",
        }
    }

    /// The setting `PLAYMAT_AI_POLICY`, `playouts` when unset or unknown
    pub fn from_env() -> PolicyKind {
        std::env::var("PLAYMAT_AI_POLICY").ok().and_then(|v| v.parse().ok()).unwrap_or_default()
    }

    pub fn build(self, seat: Seat) -> Box<dyn DecisionPolicy> {
        match self {
            PolicyKind::Playouts => Box::new(crate::search::Playouts),
            PolicyKind::Mcts => Box::new(Mcts),
            PolicyKind::Quick => Box::new(Quick),
            PolicyKind::Random => Box::new(Random::new(0x5eed_0000 + seat as u64)),
        }
    }
}

impl FromStr for PolicyKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "" | "playouts" | "search" | "default" => Ok(PolicyKind::Playouts),
            "mcts" => Ok(PolicyKind::Mcts),
            "quick" => Ok(PolicyKind::Quick),
            "random" => Ok(PolicyKind::Random),
            other => Err(format!("unknown AI policy {other:?} (playouts, mcts, quick, random)")),
        }
    }
}

/// One question for the AI's seat
pub struct Decision<'a> {
    pub game: &'a Game,
    pub seat: Seat,
    pub query: &'a Query,
    /// what the quick answers say (`policy::Policy`), always the first candidate
    pub quick: &'a Answer,
    /// the answers worth trying, at most `config.width`
    pub candidates: &'a [Answer],
    pub config: &'a Config,
    /// where candidates can be played out; `None` before the game's first priority question
    pub root: Option<&'a Root>,
}

/// A policy's answer and the search nodes (answers in playouts) it spent
pub struct Choice {
    pub answer: Answer,
    pub nodes: u64,
}

/// Answers single decisions for one seat.
pub trait DecisionPolicy: Send {
    fn name(&self) -> &str;
    /// The answer, or `None` for the quick one
    fn decide(&mut self, d: &Decision) -> Option<Choice>;
}

/// The game at the last priority question and the answers since: what a copy replays to reach the question.
#[derive(Clone)]
pub struct Root {
    pub snapshot: Game,
    pub since: Vec<Answer>,
}

/// A played-out copy
pub struct Outcome {
    pub game: Game,
    /// answers given after the candidate (search nodes)
    pub nodes: u64,
}

impl Root {
    /// Plays `candidate` on a copy until turn `stop_turn` starts or `cap` answers are given, both seats on the quick
    /// answers. With `shuffle`, the libraries are shuffled first (the AI can't know their order) and one answer in
    /// sixteen of the quick ones is a random candidate instead. `None` when the copy didn't reach the question again.
    pub fn play_out(&self, candidate: &Answer, cap: u64, stop_turn: u32, shuffle: Option<u64>) -> Result<Outcome, u64> {
        let mut g = self.snapshot.clone();
        if let Some(seed) = shuffle {
            let mut rng = JavaRandom::new(seed as i64);
            for seat in 0..g.players.len() {
                let mut library = g.player(seat).library.clone();
                rng.shuffle(&mut library);
                g.player_mut(seat).library = library;
            }
            g.rng = JavaRandom::new(seed.rotate_left(17) as i64);
        }
        let mut script = self.since.clone();
        script.push(candidate.clone());
        let mut p = Playout::new(script, cap, stop_turn, shuffle);
        rules::play_from_priority_question(&mut g, &mut p, stop_turn);
        if p.diverged {
            Err(p.answers)
        } else {
            Ok(Outcome { game: g, nodes: p.answers })
        }
    }
}

/// A playout: the recorded answers back to the question, the candidate, then quick answers for everyone until the
/// turn ends or the answers run out.
struct Playout {
    script: Vec<Answer>,
    next: usize,
    policy: [Policy; 2],
    repeats: Repeats,
    answers: u64,
    cap: u64,
    stop_turn: u32,
    stopped: bool,
    /// the copy asked something other than what was recorded: its result means nothing
    diverged: bool,
    noise: Option<u64>,
}

impl Playout {
    fn new(script: Vec<Answer>, cap: u64, stop_turn: u32, noise: Option<u64>) -> Playout {
        Playout {
            script,
            next: 0,
            policy: [Policy::default(), Policy::default()],
            repeats: Repeats::default(),
            answers: 0,
            cap,
            stop_turn,
            stopped: false,
            diverged: false,
            noise: noise.map(|s| s | 1),
        }
    }
}

impl Decider for Playout {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        if self.next < self.script.len() {
            let a = self.script[self.next].clone();
            self.next += 1;
            if !q.accepts(&a) {
                self.diverged = true;
                self.stopped = true;
                return Answer::Stop;
            }
            return a;
        }
        self.answers += 1;
        if self.answers > self.cap || game.turn >= self.stop_turn {
            self.stopped = true;
            return Answer::Stop;
        }
        let mut a = self.policy[seat.min(1)].answer(game, seat, q);
        if let Some(state) = self.noise.as_mut() {
            let r = xorshift(state);
            if r.is_multiple_of(noise_rate()) && !matches!(q, Query::PlayMana { .. }) {
                let options = candidates(game, seat, q, &a, usize::MAX);
                a = options[(r >> 8) as usize % options.len()].clone();
            }
        }
        if self.repeats.is_repeat(seat, q) {
            let options = candidates(game, seat, q, &a, usize::MAX);
            return self.repeats.guard(seat, q, a, &options);
        }
        self.repeats.guard(seat, q, a, &[])
    }

    fn stopped(&self) -> bool {
        self.stopped
    }
}

/// one quick answer in this many is random in a shuffled playout (`PLAYMAT_AI_MCTS_NOISE`, 0 for none)
fn noise_rate() -> u64 {
    static RATE: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *RATE.get_or_init(|| {
        std::env::var("PLAYMAT_AI_MCTS_NOISE").ok().and_then(|v| v.parse().ok()).map_or(16, |v: u64| {
            if v == 0 {
                u64::MAX
            } else {
                v
            }
        })
    })
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// The quick answers alone
pub struct Quick;

impl DecisionPolicy for Quick {
    fn name(&self) -> &str {
        "quick"
    }
    fn decide(&mut self, _: &Decision) -> Option<Choice> {
        None
    }
}

/// A random candidate (seeded, so a game replays the same)
pub struct Random {
    state: u64,
}

impl Random {
    pub fn new(seed: u64) -> Random {
        Random { state: seed | 1 }
    }
}

impl DecisionPolicy for Random {
    fn name(&self) -> &str {
        "random"
    }
    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        // every option the question lists, not only the first `width`
        let all = candidates(d.game, d.seat, d.query, d.quick, usize::MAX);
        let i = (xorshift(&mut self.state) >> 8) as usize % all.len().max(1);
        all.get(i).cloned().map(|answer| Choice { answer, nodes: 0 })
    }
}

/// Monte Carlo tree search at the root: UCB1 over the candidates, each sample a playout on a copy whose libraries are
/// shuffled (and whose quick answers are now and then random), run on `config.threads` threads that each spend
/// `config.budget / threads` nodes and pool their counts at the end. A candidate's first sample is the plain playout
/// `Playouts` would make, on one of the threads, so MCTS sees at least what the default search sees.
pub struct Mcts;

/// UCB1's exploration constant, on values in 0..1
const EXPLORE: f64 = 0.5;
/// score difference (eval units) that counts as clearly better: a few life points or a good creature
const SCALE: f64 = 2500.0;

impl DecisionPolicy for Mcts {
    fn name(&self) -> &str {
        "mcts"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        let root = d.root?;
        let k = d.candidates.len();
        let cfg = d.config;
        let threads = cfg.threads.clamp(1, 16);
        let base = eval::evaluate(d.game, d.seat) as f64;
        let stop = d.game.turn + 1 + cfg.extra_turns;
        let budget = (cfg.budget / threads as u64).max(1);
        let cap = (cfg.budget / k as u64).clamp(50, cfg.playout_cap);
        let seed = (d.game.turn as u64) << 32 ^ (root.since.len() as u64) << 8 ^ d.seat as u64;
        let cands = d.candidates;
        let seat = d.seat;
        // a game copy can go to another thread but not be shared (its `Cell`s): each worker gets its own root
        let results: Vec<Stats> = if threads == 1 {
            vec![worker(root, cands, seat, base, budget, cap, stop, 0, 1, seed)]
        } else {
            std::thread::scope(|s| {
                let handles: Vec<_> = (0..threads)
                    .map(|t| {
                        let mine = root.clone();
                        s.spawn(move || worker(&mine, cands, seat, base, budget, cap, stop, t, threads, seed))
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().expect("mcts worker")).collect()
            })
        };
        let mut visits = vec![0u64; k];
        let mut value = vec![0f64; k];
        let mut nodes = 0;
        for r in &results {
            nodes += r.nodes;
            for i in 0..k {
                visits[i] += r.visits[i];
                value[i] += r.value[i];
            }
        }
        let best = (0..k).filter(|i| visits[*i] > 0).max_by(|a, b| {
            let (ma, mb) = (value[*a] / visits[*a] as f64, value[*b] / visits[*b] as f64);
            ma.total_cmp(&mb).then(visits[*a].cmp(&visits[*b])).then(b.cmp(a))
        })?;
        Some(Choice { answer: d.candidates[best].clone(), nodes })
    }
}

struct Stats {
    visits: Vec<u64>,
    value: Vec<f64>,
    nodes: u64,
}

#[allow(clippy::too_many_arguments)]
fn worker(
    root: &Root,
    candidates: &[Answer],
    seat: Seat,
    base: f64,
    budget: u64,
    cap: u64,
    stop: u32,
    thread: usize,
    threads: usize,
    seed: u64,
) -> Stats {
    let k = candidates.len();
    let mut s = Stats { visits: vec![0; k], value: vec![0.0; k], nodes: 0 };
    // a candidate whose copy goes another way can't be judged (dead), or only without shuffling (plain)
    let mut dead = vec![false; k];
    let mut plain = vec![false; k];
    // judged once without shuffling, all a plain candidate can give: kept, not sampled again
    let mut fixed = vec![false; k];
    let mut tried = vec![false; k];
    let mut iter = 0u64;
    while s.nodes < budget {
        iter += 1;
        let total: u64 = s.visits.iter().sum();
        // first visits: this thread's share of the plain playouts, then the rest in order
        let pick = (0..k)
            .filter(|i| !dead[*i] && !fixed[*i] && !tried[*i])
            .min_by_key(|i| (*i % threads != thread, *i))
            .or_else(|| {
                (0..k).filter(|i| !dead[*i] && !fixed[*i]).max_by(|a, b| {
                    let ucb = |i: usize| {
                        let n = s.visits[i].max(1) as f64;
                        s.value[i] / n + EXPLORE * ((total.max(1) as f64).ln() / n).sqrt()
                    };
                    ucb(*a).total_cmp(&ucb(*b)).then(b.cmp(a))
                })
            });
        let Some(i) = pick else { break };
        let first = !tried[i];
        tried[i] = true;
        let shuffle = if (first && i % threads == thread) || plain[i] {
            None
        } else {
            Some(seed ^ (thread as u64) << 48 ^ iter.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        };
        match root.play_out(&candidates[i], cap, stop, shuffle) {
            Ok(o) => {
                s.nodes += o.nodes.max(1);
                let score = eval::evaluate(&o.game, seat) as f64;
                let v = 0.5 + 0.5 * ((score - base) / SCALE).tanh();
                s.visits[i] += 1;
                s.value[i] += v;
                fixed[i] = plain[i];
            }
            Err(n) => {
                s.nodes += n.max(1);
                if shuffle.is_some() {
                    plain[i] = true;
                } else {
                    dead[i] = true;
                }
            }
        }
        if (0..k).all(|i| dead[i] || fixed[i]) {
            break;
        }
    }
    s
}
