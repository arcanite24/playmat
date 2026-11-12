//! The search: each question the AI's seat is asked, it tries every sensible answer on a copy of the game, plays each
//! copy on to the end of the turn with the quick answers (`Policy`) for both seats, and keeps the answer whose game
//! scores best (`eval`). Copies are cheap (the engine's state is shared until changed), so this is a lot of whole
//! turns per decision instead of a deep tree of single actions.
//!
//! A copy can only be resumed where the engine asks `Query::Priority` (`rules::play_from_priority_question`), so the
//! table keeps the game as it was at the last priority question and every answer given since: a copy replays those
//! answers to get back to the question being searched.

use crate::decision::{Choice, Decision, DecisionPolicy, PolicyKind, Root};
use crate::eval;
use crate::policy::{Policy, Repeats};
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::state::{Game, Seat, Zone};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct Config {
    /// answers a search may spend in its playouts, over all its candidates
    pub budget: u64,
    /// most candidates tried per question (the quick answer's pick first)
    pub width: usize,
    /// turns a playout goes past the current one (0: to the end of this turn)
    pub extra_turns: u32,
    /// answers one playout may give before it's cut off
    pub playout_cap: u64,
    /// who answers the seat's decisions (`PLAYMAT_AI_POLICY`)
    pub policy: PolicyKind,
    /// threads a search may use (`Mcts`; `PLAYMAT_AI_THREADS`)
    pub threads: usize,
}

impl Default for Config {
    /// The settings `PLAYMAT_AI_POLICY` and `PLAYMAT_AI_THREADS` (default: playouts, and up to 4 threads) apply here
    fn default() -> Self {
        let threads = std::env::var("PLAYMAT_AI_THREADS")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get().min(4)));
        Config {
            budget: 6000,
            width: 12,
            extra_turns: 0,
            playout_cap: 1500,
            policy: PolicyKind::from_env(),
            threads: threads.max(1),
        }
    }
}

/// One searched decision, for the bench
#[derive(Clone, Debug)]
pub struct DecisionStat {
    pub game: usize,
    pub seat: Seat,
    pub turn: u32,
    pub kind: &'static str,
    pub nanos: u128,
    pub nodes: u64,
    pub options: usize,
    pub permanents: usize,
}

/// A seat's decider and the config kind it was built from (`None` when set with `set_policy`).
type SeatDecider = (Option<PolicyKind>, Box<dyn DecisionPolicy>);

/// Answers both seats of a game: searched for AI seats, quick for the others (`ai[seat] == None`).
pub struct Table {
    pub ai: Vec<Option<Config>>,
    policy: Vec<Policy>,
    repeats: Repeats,
    /// each seat's decider, built from its config's `policy` (or set with `set_policy`)
    deciders: Vec<Option<SeatDecider>>,
    /// the game at the last priority question, and the answers since
    snapshot: Option<Game>,
    since: Vec<Answer>,
    pub stats: Vec<DecisionStat>,
    pub answers: u64,
    /// the game stops after this many answers (a guard against a game that never ends)
    pub max_answers: u64,
    stop: bool,
}

impl Table {
    pub fn new(ai: Vec<Option<Config>>) -> Table {
        let n = ai.len();
        Table {
            ai,
            policy: (0..n).map(|_| Policy::default()).collect(),
            repeats: Repeats::default(),
            deciders: (0..n).map(|_| None).collect(),
            snapshot: None,
            since: Vec::new(),
            stats: Vec::new(),
            answers: 0,
            max_answers: 200_000,
            stop: false,
        }
    }

    /// Has `policy` answer `seat`'s decisions instead of the one its config names (a Jev, LLM or ONNX decider).
    /// The seat still needs a config: it gives the policy its budget and width.
    pub fn set_policy(&mut self, seat: Seat, policy: Box<dyn DecisionPolicy>) {
        self.deciders[seat] = Some((None, policy));
    }

    /// The name of the policy answering `seat`'s decisions, `None` for a seat on the quick answers alone
    pub fn policy_name(&self, seat: Seat) -> Option<String> {
        let cfg = self.ai.get(seat).cloned().flatten()?;
        Some(match self.deciders.get(seat).and_then(|d| d.as_ref()) {
            Some((None, p)) => p.name().to_string(),
            _ => cfg.policy.name().to_string(),
        })
    }

    /// Whether the game was cut off by `max_answers`
    pub fn stopped_early(&self) -> bool {
        self.stop
    }

    /// Notes an answer given by someone else (a mirrored game's other seat), so later searches can replay it.
    pub fn record(&mut self, game: &Game, q: &Query, a: &Answer) {
        if matches!(q, Query::Priority { .. }) {
            self.snapshot = Some(playout_copy(game));
            self.since.clear();
        }
        self.since.push(a.clone());
    }

    /// The AI's answer for `seat` (searched when the seat has a config), without recording it.
    pub fn decide(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        if matches!(q, Query::Priority { .. }) {
            self.snapshot = Some(playout_copy(game));
            self.since.clear();
        }
        let filtered;
        let q = match q {
            Query::Priority { playable } => {
                let pol = &mut self.policy[seat];
                filtered = Query::Priority { playable: pol.playable(game, seat, playable) };
                &filtered
            }
            _ => q,
        };
        let quick = match q {
            Query::Priority { playable } => self.policy[seat].priority(game, seat, playable),
            _ => self.policy[seat].answer(game, seat, q),
        };
        let chosen = self.search(game, seat, q, quick);
        if matches!(q, Query::Priority { .. }) {
            self.policy[seat].played(&chosen);
        }
        chosen
    }

    fn search(&mut self, game: &Game, seat: Seat, q: &Query, quick: Answer) -> Answer {
        let Some(cfg) = self.ai.get(seat).cloned().flatten() else { return quick };
        let candidates = candidates(game, seat, q, &quick, cfg.width);
        if candidates.len() < 2 {
            return quick;
        }
        let root = self.snapshot.clone().map(|snapshot| Root { snapshot, since: self.since.clone() });
        let slot = &mut self.deciders[seat];
        if !matches!(slot, Some((None, _))) && slot.as_ref().map(|(k, _)| *k) != Some(Some(cfg.policy)) {
            *slot = Some((Some(cfg.policy), cfg.policy.build(seat)));
        }
        let decider = &mut slot.as_mut().unwrap().1;
        let start = Instant::now();
        let d = Decision {
            game,
            seat,
            query: q,
            quick: &quick,
            candidates: &candidates,
            config: &cfg,
            root: root.as_ref(),
        };
        let choice = decider.decide(&d);
        let nodes = choice.as_ref().map_or(0, |c| c.nodes);
        self.stats.push(DecisionStat {
            game: 0,
            seat,
            turn: game.turn,
            kind: kind(q),
            nanos: start.elapsed().as_nanos(),
            nodes,
            options: candidates.len(),
            permanents: game.battlefield.len(),
        });
        match choice {
            Some(c) if q.accepts(&c.answer) => c.answer,
            _ => quick,
        }
    }
}

/// The default search: each candidate played once to the end of the turn (`extra_turns` more), the one whose game
/// scores best kept; `budget` split evenly over the candidates.
pub struct Playouts;

impl DecisionPolicy for Playouts {
    fn name(&self) -> &str {
        "playouts"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        let root = d.root?;
        let cfg = d.config;
        let stop = d.game.turn + 1 + cfg.extra_turns;
        let per = (cfg.budget / d.candidates.len() as u64).clamp(50, cfg.playout_cap);
        let mut nodes = 0;
        let mut best = (i64::MIN, d.quick.clone());
        for c in d.candidates.iter() {
            if nodes >= cfg.budget {
                break;
            }
            match root.play_out(c, per, stop, None) {
                Ok(o) => {
                    nodes += o.nodes;
                    let score = eval::evaluate(&o.game, d.seat);
                    if score > best.0 {
                        best = (score, c.clone());
                    }
                }
                Err(n) => nodes += n,
            }
        }
        Some(Choice { answer: best.1, nodes })
    }
}

impl Decider for Table {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        self.answers += 1;
        if self.answers > self.max_answers {
            self.stop = true;
            return Answer::Stop;
        }
        let a = self.decide(game, seat, q);
        let options = candidates(game, seat, q, &a, usize::MAX);
        let a = self.repeats.guard(seat, q, a, &options);
        if std::env::var_os("PLAYMAT_AI_DEBUG").is_some() {
            let name = match &a {
                Answer::Obj(id) | Answer::Ability(id, _) => {
                    format!(" ({} in {:?})", game.obj(*id).def.name, game.obj(*id).zone)
                }
                _ => String::new(),
            };
            eprintln!("turn {} {:?} seat {seat} {:?} -> {:?}{name}", game.turn, game.step, q, a);
        }
        self.since.push(a.clone());
        a
    }

    fn stopped(&self) -> bool {
        self.stop
    }
}

fn kind(q: &Query) -> &'static str {
    match q {
        Query::Priority { .. } => "priority",
        Query::Target { .. } => "target",
        Query::Attackers { .. } => "attack",
        Query::Blockers { .. } => "block",
        Query::BlockTarget { .. } => "block target",
        Query::Cards { .. } => "cards",
        Query::ChooseMode { .. } => "mode",
        Query::ChooseAbility { .. } => "ability",
        Query::YesNo { .. } => "yes/no",
        Query::Amount { .. } => "x",
        _ => "other",
    }
}

/// The answers worth trying, the quick one first.
pub(crate) fn candidates(game: &Game, seat: Seat, q: &Query, quick: &Answer, width: usize) -> Vec<Answer> {
    let mut out = vec![quick.clone()];
    let mut add = |a: Answer| {
        if !out.contains(&a) {
            out.push(a);
        }
    };
    match q {
        Query::Priority { playable } => {
            add(Answer::Bool(false));
            // one card of each name (two copies in hand are the same choice)
            let mut seen: Vec<(String, Zone)> = Vec::new();
            for id in playable {
                let o = game.obj(*id);
                if o.controller != seat && o.owner != seat {
                    continue;
                }
                // a land's mana on its own does nothing a payment won't do
                if o.zone == Zone::Battlefield && o.def.abilities.iter().all(|a| a.mana().is_some()) {
                    continue;
                }
                let key = (o.name().to_string(), o.zone);
                if o.zone == Zone::Hand && seen.contains(&key) {
                    continue;
                }
                seen.push(key);
                add(Answer::Obj(*id));
            }
        }
        Query::Target { players, objs, required } => {
            for p in players {
                add(Answer::Player(*p));
            }
            for o in objs {
                add(Answer::Obj(*o));
            }
            // done only with nothing to pick: Java's dialogs re-ask (or cancel the cast) where this engine
            // would take a done, so with options the AI always picks
            let _ = required;
            if players.is_empty() && objs.is_empty() {
                add(Answer::Bool(false));
            }
        }
        Query::Attackers { creatures } => {
            add(Answer::Bool(true));
            add(Answer::Text("special".into()));
            for c in creatures {
                add(Answer::Obj(*c));
            }
        }
        Query::Blockers { creatures } => {
            add(Answer::Bool(true));
            for c in creatures {
                add(Answer::Obj(*c));
            }
        }
        Query::BlockTarget { attackers } => {
            for a in attackers {
                add(Answer::Obj(*a));
            }
        }
        Query::Cards { objs, required } => {
            for o in objs {
                add(Answer::Obj(*o));
            }
            if !required {
                add(Answer::Bool(false));
            }
        }
        Query::ChooseMode { modes } => {
            for m in modes {
                add(Answer::Mode(*m));
            }
        }
        Query::ChooseAbility { source, abilities } => {
            for i in abilities {
                add(Answer::Ability(*source, *i));
            }
        }
        Query::YesNo { message } if !message.contains("mana pool") => {
            add(Answer::Bool(true));
            add(Answer::Bool(false));
        }
        Query::Amount { min, max } => {
            let top = (*max).min(*min + 8);
            for x in *min..=top {
                add(Answer::Int(x));
            }
        }
        _ => {}
    }
    out.truncate(width.max(1));
    out
}

/// The game a search plays out from: without the game log and the events, which only the real game keeps
fn playout_copy(game: &Game) -> Game {
    let mut g = game.clone();
    g.events = None;
    g.log = None;
    g
}
