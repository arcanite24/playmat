//! The search: each question the AI's seat is asked, it tries every sensible answer on a copy of the game, plays each
//! copy on to the end of the turn with the quick answers (`Policy`) for both seats, and keeps the answer whose game
//! scores best (`eval`). Copies are cheap (the engine's state is shared until changed), so this is a lot of whole
//! turns per decision instead of a deep tree of single actions.
//!
//! A copy can only be resumed where the engine asks `Query::Priority` (`rules::play_from_priority_question`), so the
//! table keeps the game as it was at the last priority question and every answer given since: a copy replays those
//! answers to get back to the question being searched.

use crate::eval;
use crate::policy::{Policy, Repeats};
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::rules;
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
}

impl Default for Config {
    fn default() -> Self {
        Config { budget: 6000, width: 12, extra_turns: 0, playout_cap: 1500 }
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

/// Answers both seats of a game: searched for AI seats, quick for the others (`ai[seat] == None`).
pub struct Table {
    pub ai: Vec<Option<Config>>,
    policy: Vec<Policy>,
    repeats: Repeats,
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
            snapshot: None,
            since: Vec::new(),
            stats: Vec::new(),
            answers: 0,
            max_answers: 200_000,
            stop: false,
        }
    }

    /// Whether the game was cut off by `max_answers`
    pub fn stopped_early(&self) -> bool {
        self.stop
    }

    /// Notes an answer given by someone else (a mirrored game's other seat), so later searches can replay it.
    pub fn record(&mut self, game: &Game, q: &Query, a: &Answer) {
        if matches!(q, Query::Priority { .. }) {
            self.snapshot = Some(game.clone());
            self.since.clear();
        }
        self.since.push(a.clone());
    }

    /// The AI's answer for `seat` (searched when the seat has a config), without recording it.
    pub fn decide(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        if matches!(q, Query::Priority { .. }) {
            self.snapshot = Some(game.clone());
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
        let Some(snapshot) = self.snapshot.clone() else { return quick };
        let start = Instant::now();
        let per = (cfg.budget / candidates.len() as u64).clamp(50, cfg.playout_cap);
        let mut nodes = 0;
        let mut best = (i64::MIN, quick.clone());
        for c in candidates.iter() {
            if nodes >= cfg.budget {
                break;
            }
            let mut g = snapshot.clone();
            let mut script = self.since.clone();
            script.push(c.clone());
            let mut p = Playout::new(script, per, game.turn + 1 + cfg.extra_turns);
            rules::play_from_priority_question(&mut g, &mut p, game.turn + 1 + cfg.extra_turns);
            nodes += p.answers;
            if p.diverged {
                continue;
            }
            let score = eval::evaluate(&g, seat);
            if score > best.0 {
                best = (score, c.clone());
            }
        }
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
        best.1
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
            eprintln!("turn {} {:?} seat {seat} {:?} -> {:?}", game.turn, game.step, q, a);
        }
        self.since.push(a.clone());
        a
    }

    fn stopped(&self) -> bool {
        self.stop
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
}

impl Playout {
    fn new(script: Vec<Answer>, cap: u64, stop_turn: u32) -> Playout {
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
        let a = self.policy[seat.min(1)].answer(game, seat, q);
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
