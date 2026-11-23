//! The search: each question the AI's seat is asked, it tries every sensible answer on a copy of the game, plays each
//! copy on to the end of the turn with the quick answers (`Policy`) for both seats, and keeps the answer whose game
//! scores best (`eval`). Copies are cheap (the engine's state is shared until changed), so this is a lot of whole
//! turns per decision instead of a deep tree of single actions.
//!
//! A copy can only be resumed where the engine asks `Query::Priority` (`rules::play_from_priority_question`), so the
//! table keeps the game as it was at the last priority question and every answer given since: a copy replays those
//! answers to get back to the question being searched.

use crate::decision::{Choice, Decision, DecisionPolicy, Hidden, PolicyKind, Root};
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
    /// playouts start from hands and libraries sampled from what the seat can't see (`hidden`), not the real game
    pub hide: bool,
    /// playouts per candidate (each on its own sample when `hide`), spent by sequential halving
    pub samples: usize,
    /// the playouts' quick answers: 1 the first ones, 2 with instant-speed play and abilities (`policy`)
    pub playout: u8,
    /// the score: 1 Java's `GameStateEvaluator2`, 2 the one fitted to game results (`eval::fitted`)
    pub eval: u8,
    /// attacks and blocks chosen as whole plans (`combat`), not one creature at a time
    pub plans: bool,
    /// keep or mulligan by playing sample hands out (`mulligan`), not by counting lands
    pub mulligan_sim: bool,
    /// chance per thousand that a searched question gets a random candidate instead (lower skill levels; 0 never)
    pub blunder: u32,
    /// the network a `net` or `netsearch` policy, or `eval` 3, uses: a `.pmnet` path, `None` the built-in one
    pub net: Option<String>,
    /// the network policy's temperature, per thousand: 0 always its best answer, 1000 a sample of its policy
    pub temp: u32,
    /// `netsearch`: how far the searched values move the network's policy, per thousand (`selfplay::NetSearch`)
    pub eta: u32,
    /// a network (`.pmnet` path, or `builtin`) that answers the playouts' questions instead of the quick answers;
    /// `None` the quick answers
    pub rollout: Option<String>,
    /// `gumbel`: how many candidates the search plays out (the likelier ones under Gumbel-top-k)
    pub consider: usize,
    /// `ismcts`: PUCT's exploration constant, per thousand
    pub cpuct: u32,
    /// `ismcts`: at most this many iterations (playouts) a decision, whatever the budget
    pub iters: u32,
    /// `ismcts`: weight per thousand on the expanded leaf's neural value vs rollout eval (0: rollout only, 1000: pure value-leaf, no rollout)
    pub leaf_blend: u32,
    /// `ismcts`: use Gumbel-top-k sequential halving at the root and completed-Q policy targets
    pub gumbel_root: bool,
    /// apply `temp` only on turns `<= temp_turn` (0: apply on all turns)
    pub temp_turn: u32,
    /// `ismcts`: optional wall-clock cap in ms per decision (0: disabled, deterministic iteration count)
    pub time_ms: u32,
    /// `ismcts` + `gumbel_root`: Q-value scaling factor `c_scale` per thousand (default 250 -> 0.25)
    pub c_scale: u32,
    /// `ismcts` + `gumbel_root`: root-parallel determinization worker threads (1 = sequential; >1 = parallel across worlds)
    pub par: u32,
    /// `ismcts` + `gumbel_root`: independent tree per determinized world (eliminates strategy fusion even at par=1)
    pub multi_tree: bool,
    /// `ismcts` + pure value-leaf: use cached neural policy (not heuristic `quick`) while resolving mid-combat/stack quiescence
    pub quiesce_net: bool,
    /// keep/mulligan and London bottom using the neural value head (`net.state`) instead of heuristic/goldfish rules
    pub mulligan_net: bool,
}

impl Default for Config {
    /// The settings `PLAYMAT_AI_POLICY` and `PLAYMAT_AI_THREADS` (default: playouts, and up to 4 threads) apply here
    fn default() -> Self {
        let threads = std::env::var("PLAYMAT_AI_THREADS")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get().min(4)));
        let policy = PolicyKind::from_env();
        // the network's search as measured (#390 Track B): on samples of what the seat can't see, scored by the
        // network's value
        let netsearch = matches!(policy, PolicyKind::NetSearch | PolicyKind::Gumbel | PolicyKind::Ismcts);
        Config {
            budget: 6000,
            width: 12,
            // one turn past this one, the v2 playouts and the fitted score: each SPRT-accepted on constructed-2
            // (#390; docs/rust/Ai.md)
            extra_turns: 1,
            playout_cap: 1500,
            policy,
            threads: threads.max(1),
            hide: netsearch,
            samples: if netsearch { 4 } else { 1 },
            playout: 2,
            eval: if netsearch { 3 } else { 2 },
            plans: false,
            mulligan_sim: false,
            blunder: 0,
            net: None,
            temp: 0,
            eta: 10_000,
            rollout: None,
            consider: 4,
            cpuct: 300,
            iters: 256,
            leaf_blend: 0,
            gumbel_root: false,
            temp_turn: 0,
            time_ms: 0,
            c_scale: 250,
            par: 1,
            multi_tree: false,
            quiesce_net: false,
            mulligan_net: false,
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
    /// what each seat has seen, for searches that sample what they can't (`Config::hide`)
    hidden: Vec<Hidden>,
    /// each seat's combat plan still to give (`Config::plans`)
    plan: Vec<std::collections::VecDeque<Answer>>,
    pub stats: Vec<DecisionStat>,
    pub answers: u64,
    /// the game stops after this many answers (a guard against a game that never ends)
    pub max_answers: u64,
    stop: bool,
}

impl Table {
    pub fn new(ai: Vec<Option<Config>>) -> Table {
        let n = ai.len();
        let policy = ai.iter().map(|c| Policy::version(c.as_ref().map_or(1, |c| c.playout))).collect();
        Table {
            hidden: (0..n).map(Hidden::new).collect(),
            plan: vec![Default::default(); n],
            ai,
            policy,
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
        self.observe(game);
        if matches!(q, Query::Priority { .. }) {
            self.snapshot = Some(playout_copy(game));
            self.since.clear();
        }
        self.since.push(a.clone());
    }

    /// Notes what the seats that search on samples see now
    fn observe(&mut self, game: &Game) {
        for (seat, cfg) in self.ai.iter().enumerate() {
            if cfg.as_ref().is_some_and(|c| c.hide) {
                self.hidden[seat].observe(game);
            }
        }
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
        let combat = matches!(q, Query::Attackers { .. } | Query::Blockers { .. } | Query::BlockTarget { .. });
        if !combat {
            self.plan[seat].clear();
        } else if let Some(a) = self.plan[seat].pop_front() {
            return a;
        }
        if cfg.plans && cfg.policy == PolicyKind::Playouts && !matches!(q, Query::BlockTarget { .. }) {
            if let Some(a) = self.plan_combat(game, seat, q, &quick, &cfg) {
                return a;
            }
        }
        let candidates = candidates(game, seat, q, &quick, cfg.width);
        if candidates.len() < 2 {
            return quick;
        }
        // a lower skill's mistakes: now and then a candidate at random instead of the search (never a payment)
        if cfg.blunder > 0 && !matches!(q, Query::PlayMana { .. }) {
            let r = self.roll();
            if r % 1000 < cfg.blunder as u64 {
                return candidates[(r >> 10) as usize % candidates.len()].clone();
            }
        }
        let hidden = cfg.hide.then(|| self.hidden[seat].clone());
        let root = self.snapshot.clone().map(|snapshot| Root {
            snapshot,
            since: self.since.clone(),
            hidden,
            playout: cfg.playout,
            rollout: cfg.rollout.as_ref().map(|p| crate::net::of(&Some(p.clone()))),
        });
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

/// The default search: each candidate played to the end of the turn (`extra_turns` more) and the one whose game
/// scores best kept. With one sample of the real game (`samples` 1, no `hide`), each candidate is played once and
/// `budget` is split evenly over them. Otherwise each candidate is played on `samples` copies on average, spent by
/// sequential halving (every candidate on the same first copies, then the better half on more, and so on), and
/// scores are averaged as values in 0..1 (`eval::value`); the copies are samples of what the seat can't see when
/// `hide` is on.
pub struct Playouts;

impl DecisionPolicy for Playouts {
    fn name(&self) -> &str {
        "playouts"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        let root = d.root?;
        let cfg = d.config;
        let stop = d.game.turn + 1 + cfg.extra_turns;
        let k = d.candidates.len();
        let per = (cfg.budget / k as u64).clamp(50, cfg.playout_cap);
        if cfg.samples <= 1 && root.hidden.is_none() {
            let mut nodes = 0;
            let mut best = (i64::MIN, d.quick.clone());
            for c in d.candidates.iter() {
                if nodes >= cfg.budget {
                    break;
                }
                match root.play_out(c, per, stop, None) {
                    Ok(o) => {
                        nodes += o.nodes;
                        let score = eval::score_with(&o.game, d.seat, cfg);
                        if score > best.0 {
                            best = (score, c.clone());
                        }
                    }
                    Err(n) => nodes += n,
                }
            }
            return Some(Choice { answer: best.1, nodes });
        }
        let plans: Vec<Vec<Answer>> = d.candidates.iter().map(|c| vec![c.clone()]).collect();
        let (best, nodes) = sampled(root, d.game, d.seat, cfg, &plans, per, stop);
        Some(Choice { answer: d.candidates[best].clone(), nodes })
    }
}

/// Plays each plan (a candidate's answers in order) on `config.samples` copies on average by sequential halving, and
/// gives the best plan's index and the nodes spent. The copies are samples of what `seat` can't see when the root
/// has `hidden`, else the real game first and library shuffles after.
pub fn sampled(
    root: &Root,
    game: &Game,
    seat: Seat,
    cfg: &Config,
    plans: &[Vec<Answer>],
    per: u64,
    stop: u32,
) -> (usize, u64) {
    let base = eval::score_with(game, seat, cfg);
    let seed = ((game.turn as u64) << 40) ^ ((root.since.len() as u64) << 16) ^ seat as u64 ^ 0x2545_f491;
    let samples = cfg.samples.max(1);
    let mut worlds: Vec<Option<Game>> = Vec::new();
    let mut world = |j: usize| -> Game {
        if worlds.len() <= j {
            worlds.resize(j + 1, None);
        }
        worlds[j]
            .get_or_insert_with(|| {
                if root.hidden.is_none() && j == 0 {
                    root.snapshot.clone()
                } else {
                    root.sample_or_shuffle(seed ^ (j as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15))
                }
            })
            .clone()
    };
    halving(plans.len(), samples, cfg.budget * samples as u64, |i, j| {
        let g = world(j);
        match root.play_on(g, &plans[i], per, stop) {
            Ok(o) => Some((eval::value_with(&o.game, seat, base, cfg), o.nodes)),
            // a sample the copy can't replay to the question: the real game instead
            Err(_) => root
                .play_on(root.snapshot.clone(), &plans[i], per, stop)
                .ok()
                .map(|o| (eval::value_with(&o.game, seat, base, cfg), o.nodes)),
        }
    })
}

/// Sequential halving over `k` candidates with about `samples` playouts each on average: every round gives the
/// candidates still in the same number of new samples (sample j is the same copy for all, so they're compared on the
/// same games), then keeps the better half by mean value. `play(i, j)` plays candidate i on sample j: its value and
/// nodes, or `None` when it can't be played (the candidate is out). Stops early once `budget` nodes are spent.
/// Returns the best candidate (the most sampled, then the best mean, then the first: the quick answer) and the nodes
/// spent.
pub fn halving(
    k: usize,
    samples: usize,
    budget: u64,
    mut play: impl FnMut(usize, usize) -> Option<(f64, u64)>,
) -> (usize, u64) {
    let mut sum = vec![0f64; k];
    let mut count = vec![0usize; k];
    let mut out = vec![false; k];
    let mut alive: Vec<usize> = (0..k).collect();
    let rounds = (usize::BITS - (k.max(2) - 1).leading_zeros()) as usize;
    let total = samples * k;
    let mut nodes = 0u64;
    let mut next = 0usize;
    'rounds: for _ in 0..rounds {
        let n = (total / (rounds * alive.len())).max(1);
        for j in next..next + n {
            for &i in &alive {
                if out[i] {
                    continue;
                }
                match play(i, j) {
                    Some((v, used)) => {
                        sum[i] += v;
                        count[i] += 1;
                        nodes += used.max(1);
                    }
                    None => out[i] = true,
                }
                if nodes >= budget {
                    break 'rounds;
                }
            }
        }
        next += n;
        alive.retain(|i| !out[*i] && count[*i] > 0);
        if alive.len() <= 1 {
            break;
        }
        let mean = |i: usize| sum[i] / count[i] as f64;
        alive.sort_by(|a, b| mean(*b).total_cmp(&mean(*a)).then(a.cmp(b)));
        alive.truncate(alive.len().div_ceil(2));
        alive.sort();
    }
    let best = (0..k)
        .filter(|i| !out[*i] && count[*i] > 0)
        .max_by(|a, b| {
            let (ma, mb) = (sum[*a] / count[*a] as f64, sum[*b] / count[*b] as f64);
            count[*a].cmp(&count[*b]).then(ma.total_cmp(&mb)).then(b.cmp(a))
        })
        .unwrap_or(0);
    (best, nodes)
}

impl Table {
    /// A random number for `blunder`, the same sequence for the same game
    fn roll(&mut self) -> u64 {
        let mut z = self.answers.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xB1_0DE5;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Searches whole attacks or blocks (`combat::plans`) and keeps the best plan's later answers to give next
    fn plan_combat(&mut self, game: &Game, seat: Seat, q: &Query, quick: &Answer, cfg: &Config) -> Option<Answer> {
        let plans = crate::combat::plans(game, q, quick)?;
        if plans.len() < 2 {
            return None;
        }
        let root = Root {
            snapshot: self.snapshot.clone()?,
            since: self.since.clone(),
            hidden: cfg.hide.then(|| self.hidden[seat].clone()),
            playout: cfg.playout,
            rollout: cfg.rollout.as_ref().map(|p| crate::net::of(&Some(p.clone()))),
        };
        let start = Instant::now();
        let stop = game.turn + 1 + cfg.extra_turns;
        let per = (cfg.budget / plans.len() as u64).clamp(50, cfg.playout_cap);
        let (best, nodes) = sampled(&root, game, seat, cfg, &plans, per, stop);
        self.stats.push(DecisionStat {
            game: 0,
            seat,
            turn: game.turn,
            kind: kind(q),
            nanos: start.elapsed().as_nanos(),
            nodes,
            options: plans.len(),
            permanents: game.battlefield.len(),
        });
        let mut plan: std::collections::VecDeque<Answer> = plans[best].iter().cloned().collect();
        let first = plan.pop_front()?;
        self.plan[seat] = plan;
        Some(first)
    }
}

impl Decider for Table {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        self.answers += 1;
        self.observe(game);
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
pub fn candidates(game: &Game, seat: Seat, q: &Query, quick: &Answer, width: usize) -> Vec<Answer> {
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
        Query::YesNo { message } if !message.contains("mana pool") && !crate::policy::declines(message) => {
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
