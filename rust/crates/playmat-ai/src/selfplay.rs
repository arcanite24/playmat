//! Self-play (#390 Track B): the network's policies, and the recording of games for training (`rust/rl`).
//!
//! - `NetPolicy` answers with the network's policy: its best answer, or a sample at `Config::temp`.
//! - `NetSearch` improves the network's policy at play time (L3): each of the likelier candidates is played out on
//!   samples of what the seat can't see, chosen by the belief head, and scored by the value head; the policy is then
//!   moved toward the candidates that scored well, `π'(a) ∝ π(a)·exp(η·(Q(a) − Q̄))`, as one regularized update
//!   (Ataraxos's update-equivalence search, simplified).
//! - `Recording` wraps any seat's policy and writes each decision it makes (the observation, the candidates, the
//!   choice, the policy's chances, the critic's extra view, the belief's labels) for training.

use crate::decision::{Choice, Decision, DecisionPolicy, PolicyKind};
use crate::fm::Fm;
use crate::net::{self, softmax};
use crate::obs::{self, Obs, ACT_F, BELIEF_META_F, ENT_F, GLOB_F};
use crate::search::Config;
use playmat_engine::decide::Query;
use playmat_engine::state::{Game, ObjId, Seat, Zone};
use std::io::Write;
use std::sync::{Arc, Mutex};

/// splitmix64
pub fn mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub(crate) fn uniform(state: &mut u64) -> f32 {
    (mix(state) >> 40) as f32 / (1u64 << 24) as f32
}

/// Effective sampling temperature: `cfg.temp` when `temp_turn == 0` or `turn <= temp_turn`, else 0 (greedy)
pub(crate) fn effective_temp(cfg: &Config, turn: u32) -> u32 {
    if cfg.temp_turn > 0 && turn > cfg.temp_turn {
        0
    } else {
        cfg.temp
    }
}

/// An index drawn from `p`
pub fn draw(p: &[f32], state: &mut u64) -> usize {
    let mut u = uniform(state);
    for (i, v) in p.iter().enumerate() {
        if u < *v {
            return i;
        }
        u -= v;
    }
    p.len() - 1
}

/// Questions the network leaves to the quick answers: paying mana and dividing damage
pub fn skipped(q: &Query) -> bool {
    matches!(q, Query::PlayMana { .. } | Query::MultiAmount { .. })
}

/// The network's policy
pub struct NetPolicy {
    rng: u64,
    last: Vec<f32>,
}

impl NetPolicy {
    pub fn new(seed: u64) -> NetPolicy {
        NetPolicy { rng: seed, last: Vec::new() }
    }
}

/// Picks from `p`: the most likely at temperature 0, else a sample
pub(crate) fn pick(p: &[f32], temp: u32, rng: &mut u64) -> usize {
    if temp == 0 {
        p.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map_or(0, |(i, _)| i)
    } else {
        draw(p, rng)
    }
}

impl DecisionPolicy for NetPolicy {
    fn name(&self) -> &str {
        "net"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.last.clear();
        if skipped(d.query) {
            return None;
        }
        let net = net::of(&d.config.net);
        let o = obs::observe(d.game, d.seat, d.query, d.candidates);
        let out = net.forward(&o);
        let temp = effective_temp(d.config, d.game.turn);
        let t = if temp == 0 { 1.0 } else { temp as f32 / 1000.0 };
        self.last = softmax(&out.logits, t);
        let i = pick(&self.last, temp, &mut self.rng);
        Some(Choice { answer: d.candidates[i].clone(), nodes: 0 })
    }

    fn last_policy(&self) -> Option<&[f32]> {
        (!self.last.is_empty()).then_some(&self.last[..])
    }
}

/// The network's policy improved by a search at play time
pub struct NetSearch {
    rng: u64,
    last: Vec<f32>,
    last_val: Option<f32>,
}

impl NetSearch {
    pub fn new(seed: u64) -> NetSearch {
        NetSearch { rng: seed, last: Vec::new(), last_val: None }
    }
}

/// The opponent's cards the seat can't place: its hand cards not seen and its library, with whether each is in hand
pub fn unseen(game: &Game, seat: Seat, seen: impl Fn(ObjId) -> bool) -> Vec<(ObjId, bool)> {
    let p = game.player(game.opponent(seat));
    let mut out: Vec<(ObjId, bool)> = p.hand.iter().filter(|id| !seen(**id)).map(|id| (*id, true)).collect();
    out.extend(p.library.iter().map(|id| (*id, false)));
    out
}

/// How likely the belief makes a sample's opponent hand: the sum over unseen cards of log σ(z) in hand, log σ(−z)
/// out of it
fn hand_likelihood(
    game: &Game,
    seat: Seat,
    logit: &std::collections::HashMap<u32, f32>,
    seen: &dyn Fn(ObjId) -> bool,
) -> f32 {
    let ls = |z: f32| -(1.0 + (-z).sexp()).sln();
    unseen(game, seat, seen)
        .iter()
        .map(|(id, in_hand)| {
            let z = logit.get(&obs::card_id(&game.obj(*id).def.name)).copied().unwrap_or(0.0);
            if *in_hand {
                ls(z)
            } else {
                ls(-z)
            }
        })
        .sum()
}

/// `k` samples of the game the seat might be in, deduplicated with their exact belief-draw multiplicity weights
/// (Rao-Blackwellized unbiased importance resampling, 3 drawn per kept).
pub(crate) fn sample_games_weighted(
    rng: &mut u64,
    d: &Decision,
    root: &crate::decision::Root,
    net: &net::Net,
    state: &[f32],
    k: usize,
) -> Vec<(Game, f64)> {
    let seed = ((d.game.turn as u64) << 40) ^ ((root.since.len() as u64) << 16) ^ d.seat as u64 ^ *rng;
    mix(rng);
    let Some(h) = &root.hidden else {
        return vec![(root.snapshot.clone(), 1.0)];
    };
    let opp_unseen_hand =
        root.snapshot.player(root.snapshot.opponent(d.seat)).hand.iter().filter(|id| !h.seat_saw(**id)).count();
    // When opponent has no hidden hand cards (hellbent / fully revealed), cap world count to 2 so
    // multi-tree search focuses depth instead of splitting iters across 16 near-identical trees.
    let eff_k = if opp_unseen_hand == 0 { k.clamp(1, 2) } else { k.max(1) };
    match Some(h) {
        Some(h) if net.has_belief() && opp_unseen_hand > 0 => {
            let seen = |id: ObjId| h.seat_saw(id);
            let pool: Vec<u32> = unseen(&root.snapshot, d.seat, seen)
                .iter()
                .map(|(id, _)| obs::card_id(&root.snapshot.obj(*id).def.name))
                .collect();
            let z = net.belief(state, &pool);
            let mut logit: std::collections::HashMap<u32, f32> = pool.iter().copied().zip(z).collect();
            if d.config.belief_pass {
                // Bayesian pass-likelihood adjustment: if opponent holds >= 2 untapped lands outside their own
                // main phase, gently upweight instants/flash and downweight payable sorcery-speed creatures.
                let opp = root.snapshot.opponent(d.seat);
                let opp_untapped_lands = root.snapshot.battlefield.iter().filter(|id| {
                    let o = root.snapshot.obj(**id);
                    o.controller == opp && !o.tapped && root.snapshot.has_type(**id, playmat_engine::card::CardType::Land)
                }).count();
                let opp_passed_open = root.snapshot.active != Some(opp) && opp_untapped_lands >= 2;
                if opp_passed_open {
                    for (id, _) in unseen(&root.snapshot, d.seat, seen) {
                        let def = &root.snapshot.obj(id).def;
                        let cid = obs::card_id(&def.name);
                        let is_instant = def.types.contains(&playmat_engine::card::CardType::Instant)
                            || def.abilities.contains(&playmat_engine::card::AbilityDef::FlashAbility);
                        let is_creature = def.types.contains(&playmat_engine::card::CardType::Creature) && !is_instant;
                        let mv = def.mana_value() as usize;
                        if let Some(v) = logit.get_mut(&cid) {
                            if is_instant && mv <= opp_untapped_lands {
                                *v += 0.30;
                            } else if is_creature && mv > 0 && mv <= opp_untapped_lands {
                                *v -= 0.20;
                            }
                        }
                    }
                }
            }
            let n_draw = if d.config.belief_ms { eff_k * 6 } else { eff_k * 3 };
            let threat_logits = if d.config.threat_w > 0 && net.has_threat() {
                Some(net.threat(state))
            } else {
                None
            };
            let threat_scale = d.config.threat_w as f32 / 1000.0;
            let mut drawn: Vec<(f32, Game)> = (0..n_draw)
                .map(|j| {
                    let g = root.sample(seed.wrapping_add(j as u64 * 0x9E37));
                    let mut ll = hand_likelihood(&g, d.seat, &logit, &seen);
                    if let Some(tl) = &threat_logits {
                        let y = obs::belief_meta(&g, d.seat, 0);
                        let mut t_ll = 0.0f32;
                        for k in 0..obs::BELIEF_META_F {
                            t_ll += y[k] * tl[k].clamp(-6.0, 6.0);
                        }
                        ll += threat_scale * t_ll;
                    }
                    (ll, g)
                })
                .collect();
            let m = drawn.iter().map(|(l, _)| *l).fold(f32::MIN, f32::max);
            let w: Vec<f64> = drawn.iter().map(|(l, _)| ((l - m) as f64).sexp()).collect();
            let s: f64 = w.iter().sum::<f64>().max(1e-12);
            if d.config.belief_ms {
                // Phase 3: group proposals by sorted unseen opponent-hand card-ID multiset, sum exact
                // posterior probability mass per distinct hand, and keep top-eff_k highest-posterior hands.
                let opp = root.snapshot.opponent(d.seat);
                let mut by_hand: std::collections::HashMap<Vec<u32>, (f64, usize)> = std::collections::HashMap::new();
                for (j, (_, g)) in drawn.iter().enumerate() {
                    let mut key: Vec<u32> = g
                        .player(opp)
                        .hand
                        .iter()
                        .filter(|id| !seen(**id))
                        .map(|id| obs::card_id(&g.obj(*id).def.name))
                        .collect();
                    key.sort_unstable();
                    let pj = w[j] / s;
                    by_hand.entry(key).and_modify(|e| e.0 += pj).or_insert((pj, j));
                }
                let mut ranked: Vec<(f64, usize)> = by_hand.into_values().collect();
                ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
                ranked.truncate(eff_k);
                let tot_p: f64 = ranked.iter().map(|(p, _)| *p).sum::<f64>().max(1e-12);
                // Sort descending by index so swap_remove doesn't invalidate earlier indices
                ranked.sort_by_key(|r| std::cmp::Reverse(r.1));
                let mut keep = Vec::with_capacity(ranked.len());
                for (p, idx) in ranked {
                    keep.push((drawn.swap_remove(idx).1, (p / tot_p) * eff_k as f64));
                }
                keep
            } else {
                let wf: Vec<f32> = w.iter().map(|v| (v / s) as f32).collect();
                let mut picks: Vec<usize> = (0..eff_k).map(|_| draw(&wf, rng)).collect();
                picks.sort();
                let mut groups: Vec<(usize, u32)> = Vec::new();
                for p in picks {
                    if let Some(last) = groups.last_mut() {
                        if last.0 == p {
                            last.1 += 1;
                            continue;
                        }
                    }
                    groups.push((p, 1));
                }
                let mut keep = Vec::with_capacity(groups.len());
                for (i, cnt) in groups.into_iter().rev() {
                    keep.push((drawn.swap_remove(i).1, cnt as f64));
                }
                keep
            }
        }
        _ => (0..eff_k).map(|j| (root.sample(seed.wrapping_add(j as u64 * 0x9E37)), 1.0)).collect(),
    }
}

/// `k` samples of the game the seat might be in; with a belief head, the likelier ones under it (importance
/// resampling, 3 drawn per kept)
pub(crate) fn sample_games(
    rng: &mut u64,
    d: &Decision,
    root: &crate::decision::Root,
    net: &net::Net,
    state: &[f32],
    k: usize,
) -> Vec<Game> {
    sample_games_weighted(rng, d, root, net, state, k).into_iter().map(|(g, _)| g).collect()
}

impl DecisionPolicy for NetSearch {
    fn name(&self) -> &str {
        "netsearch"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.last.clear();
        self.last_val = None;
        if skipped(d.query) {
            return None;
        }
        let cfg = d.config;
        let temp = effective_temp(cfg, d.game.turn);
        let net = net::of(&cfg.net);
        let o = obs::observe(d.game, d.seat, d.query, d.candidates);
        let out = net.forward(&o);
        let prior = softmax(&out.logits, 1.0);
        let Some(root) = d.root else {
            self.last = prior;
            let i = pick(&self.last, temp, &mut self.rng);
            return Some(Choice { answer: d.candidates[i].clone(), nodes: 0 });
        };
        // the likelier candidates, at most 4, that together hold most of the policy
        let mut order: Vec<usize> = (0..prior.len()).collect();
        order.sort_by(|a, b| prior[*b].total_cmp(&prior[*a]));
        let mut tried = Vec::new();
        let mut mass = 0.0;
        for i in order {
            if tried.len() >= 4 || (mass > 0.95 && !tried.is_empty()) {
                break;
            }
            tried.push(i);
            mass += prior[i];
        }
        if tried.len() < 2 {
            self.last = prior;
            return Some(Choice { answer: d.candidates[tried[0]].clone(), nodes: 0 });
        }
        let games = sample_games(&mut self.rng, d, root, &net, &out.h, cfg.samples.max(1));
        let stop = d.game.turn + 1 + cfg.extra_turns;
        let per = (cfg.budget / (tried.len() * games.len()) as u64).clamp(50, cfg.playout_cap);
        let base = crate::eval::score_with(d.game, d.seat, cfg);
        let mut nodes = 0;
        let mut q = vec![f32::NAN; prior.len()];
        for &i in &tried {
            let mut sum = 0.0;
            let mut n = 0;
            for g in &games {
                match root.play_on(g.clone(), &[d.candidates[i].clone()], per, stop) {
                    Ok(o) => {
                        nodes += o.nodes;
                        sum += crate::eval::value_with(&o.game, d.seat, base, cfg);
                        n += 1;
                    }
                    Err(x) => nodes += x,
                }
            }
            if n > 0 {
                q[i] = (sum / n as f64) as f32;
            }
        }
        let known: Vec<f32> = q.iter().copied().filter(|v| !v.is_nan()).collect();
        let mean = known.iter().sum::<f32>() / known.len().max(1) as f32;
        let eta = cfg.eta as f32 / 1000.0;
        // untried candidates keep their prior; tried ones are moved by how they scored against the mean
        let mut post: Vec<f32> =
            prior.iter().zip(&q).map(|(p, v)| if v.is_nan() { *p } else { p * (eta * (v - mean)).sexp() }).collect();
        let s: f32 = post.iter().sum();
        post.iter_mut().for_each(|v| *v /= s);
        let q_weighted: f32 = post.iter().zip(&q).map(|(p, v)| p * if v.is_nan() { mean } else { *v }).sum();
        if !known.is_empty() && q_weighted.is_finite() {
            self.last_val = Some(q_weighted.clamp(0.0, 1.0));
        }
        self.last = post;
        let i = pick(&self.last, temp, &mut self.rng);
        Some(Choice { answer: d.candidates[i].clone(), nodes })
    }

    fn last_policy(&self) -> Option<&[f32]> {
        (!self.last.is_empty()).then_some(&self.last[..])
    }

    fn last_value(&self) -> Option<f32> {
        self.last_val
    }
}

/// The network's policy searched by Gumbel sequential halving (Gumbel AlphaZero, Danihelka et al. 2022): `consider`
/// candidates drawn without replacement by Gumbel-top-k on the logits (no noise at temperature 0), then rounds that
/// play each survivor out on more samples and keep the better half by `g + logit + σ(Q)`. The policy it records is
/// `softmax(logit + σ(Q))`, the candidates it didn't play valued at the network's value.
pub struct Gumbel {
    rng: u64,
    last: Vec<f32>,
    last_val: Option<f32>,
}

impl Gumbel {
    pub fn new(seed: u64) -> Gumbel {
        Gumbel { rng: seed, last: Vec::new(), last_val: None }
    }
}

/// `σ(q) = (50 + most plays) · 0.1 · q`, the Gumbel AlphaZero scale for values in 0..1
pub(crate) fn sigma(q: f32, most: usize) -> f32 {
    (50.0 + most as f32) * 0.1 * q
}

impl DecisionPolicy for Gumbel {
    fn name(&self) -> &str {
        "gumbel"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.last.clear();
        self.last_val = None;
        if skipped(d.query) {
            return None;
        }
        let cfg = d.config;
        let temp = effective_temp(cfg, d.game.turn);
        let net = net::of(&cfg.net);
        let o = obs::observe(d.game, d.seat, d.query, d.candidates);
        let out = net.forward(&o);
        let n = out.logits.len();
        let Some(root) = d.root.filter(|_| n > 1) else {
            self.last = softmax(&out.logits, 1.0);
            let i = pick(&self.last, temp, &mut self.rng);
            return Some(Choice { answer: d.candidates[i].clone(), nodes: 0 });
        };
        let g: Vec<f32> =
            (0..n).map(|_| if temp == 0 { 0.0 } else { -(-(uniform(&mut self.rng).max(1e-9)).sln()).sln() }).collect();
        let mut alive: Vec<usize> = (0..n).collect();
        alive.sort_by(|a, b| (g[*b] + out.logits[*b]).total_cmp(&(g[*a] + out.logits[*a])));
        alive.truncate(cfg.consider.max(2));
        // `samples` plays per candidate on average, spread over log2(m) rounds that halve the survivors
        let m = alive.len();
        let rounds = (usize::BITS - (m - 1).leading_zeros()).max(1) as usize;
        let total = cfg.samples.max(1) * m;
        let games = sample_games(&mut self.rng, d, root, &net, &out.h, total.div_ceil(rounds).max(cfg.samples.max(1)));
        let stop = d.game.turn + 1 + cfg.extra_turns;
        let per = (cfg.budget / total as u64).clamp(50, cfg.playout_cap);
        let base = crate::eval::score_with(d.game, d.seat, cfg);
        let mut sum = vec![0.0f64; n];
        let mut plays = vec![0usize; n];
        let mut nodes = 0;
        for r in 0..rounds {
            let each = (total / (rounds * alive.len())).max(1);
            for &i in &alive {
                for k in 0..each {
                    let gk = &games[(plays[i] + k) % games.len()];
                    match root.play_on(gk.clone(), &[d.candidates[i].clone()], per, stop) {
                        Ok(res) => {
                            nodes += res.nodes;
                            sum[i] += crate::eval::value_with(&res.game, d.seat, base, cfg);
                        }
                        Err(x) => {
                            nodes += x;
                            sum[i] += 0.5;
                        }
                    }
                }
                plays[i] += each;
            }
            let most = *plays.iter().max().unwrap_or(&0);
            let key = |i: usize| g[i] + out.logits[i] + sigma((sum[i] / plays[i] as f64) as f32, most);
            alive.sort_by(|a, b| key(*b).total_cmp(&key(*a)));
            if r + 1 < rounds {
                alive.truncate(alive.len().div_ceil(2));
            }
        }
        let v = 1.0 / (1.0 + (-out.value).sexp());
        let most = *plays.iter().max().unwrap_or(&0);
        let improved: Vec<f32> = (0..n)
            .map(|i| {
                let q = if plays[i] > 0 { (sum[i] / plays[i] as f64) as f32 } else { v };
                out.logits[i] + sigma(q, most)
            })
            .collect();
        self.last = softmax(&improved, 1.0);
        if plays[alive[0]] > 0 {
            self.last_val = Some(((sum[alive[0]] / plays[alive[0]] as f64) as f32).clamp(0.0, 1.0));
        }
        Some(Choice { answer: d.candidates[alive[0]].clone(), nodes })
    }

    fn last_policy(&self) -> Option<&[f32]> {
        (!self.last.is_empty()).then_some(&self.last[..])
    }

    fn last_value(&self) -> Option<f32> {
        self.last_val
    }
}

/// One recorded decision
#[derive(Clone, Debug, Default)]
pub struct Step {
    pub seat: Seat,
    pub turn: u32,
    pub qkind: u8,
    pub obs: Obs,
    /// the critic's extra view: the opponent's hand as entities
    pub hidden: Vec<f32>,
    pub hidden_ids: Vec<u32>,
    /// the opponent's unseen cards (its hand cards the seat hasn't seen, and its library), and which are in hand
    pub pool: Vec<u32>,
    pub pool_in_hand: Vec<bool>,
    pub chosen: usize,
    /// the policy's chances at the decision (one-hot when it gave none)
    pub target: Vec<f32>,
    /// log of the chosen answer's chance under the policy that chose it
    pub logp: f32,
    /// search's estimated root win probability (0..1), or NaN if none
    pub qval: f32,
    /// v3: per-candidate search Q-value (0..1, or NaN for unvisited candidates)
    pub act_q: Vec<f32>,
    /// v3: per-candidate search visit counts
    pub act_n: Vec<u32>,
    /// v3: structured 8-float public-belief summary (`belief_meta.f16`)
    pub belief_meta: [f32; BELIEF_META_F],
}

/// The seats' recorded decisions, shared with the game's runner
pub type Sink = Arc<Mutex<Vec<Step>>>;

/// A seat's policy, recording its decisions into `sink`
pub struct Recording {
    pub inner: Box<dyn DecisionPolicy>,
    pub sink: Sink,
    /// what this seat has seen, for the belief's labels
    pub seen: crate::decision::Hidden,
    /// chance per thousand that a decision is recorded
    pub keep: u32,
    /// answers the decisions that won't be recorded, so the search is spent only on those that are (KataGo's
    /// playout cap randomization); `None` searches every decision
    pub fast: Option<Box<dyn DecisionPolicy>>,
    /// when set (`--fast ismcts:N`), unrecorded decisions run `inner` with reduced `iters = N` (KataGo playout cap)
    pub fast_iters: Option<u32>,
    pub rng: u64,
    name: String,
}

impl Recording {
    pub fn new(kind: PolicyKind, seat: Seat, sink: Sink, keep: u32, seed: u64) -> Recording {
        let inner = kind.build(seat);
        let name = inner.name().to_string();
        Recording {
            inner,
            sink,
            seen: crate::decision::Hidden::new(seat),
            keep,
            fast: None,
            fast_iters: None,
            rng: seed,
            name,
        }
    }

    /// Decisions that won't be recorded are answered by `kind` instead
    pub fn with_fast(mut self, kind: Option<PolicyKind>, seat: Seat) -> Recording {
        self.fast = kind.map(|k| k.build(seat));
        self
    }

    /// Decisions that won't be recorded run a fast mini-search with `iters` simulations (keeps tree warm & outcomes clean)
    pub fn with_fast_iters(mut self, iters: Option<u32>) -> Recording {
        self.fast_iters = iters;
        self
    }
}

impl DecisionPolicy for Recording {
    fn name(&self) -> &str {
        &self.name
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        if skipped(d.query) {
            return self.inner.decide(d);
        }
        self.seen.observe(d.game);
        if (mix(&mut self.rng) % 1000) as u32 >= self.keep {
            if let Some(fi) = self.fast_iters {
                let mut cfg = d.config.clone();
                cfg.iters = fi.max(2);
                cfg.consider = cfg.consider.min(4);
                cfg.samples = cfg.samples.min(2);
                let df = Decision {
                    game: d.game,
                    seat: d.seat,
                    query: d.query,
                    quick: d.quick,
                    candidates: d.candidates,
                    config: &cfg,
                    root: d.root,
                };
                return self.inner.decide(&df);
            }
            return match self.fast.as_mut() {
                Some(fast) => fast.decide(d),
                None => self.inner.decide(d),
            };
        }
        let choice = self.inner.decide(d);
        let answer = choice.as_ref().map_or(d.quick, |c| &c.answer);
        let Some(chosen) = d.candidates.iter().position(|c| c == answer) else { return choice };
        let k = d.candidates.len();
        let target: Vec<f32> = match self.inner.last_policy() {
            Some(p) if p.len() == k => p.to_vec(),
            _ => (0..k).map(|i| (i == chosen) as u8 as f32).collect(),
        };
        let (hidden, hidden_ids) = obs::hidden_entities(d.game, d.seat);
        let seen = |id: ObjId| self.seen.seat_saw(id);
        let pool = unseen(d.game, d.seat, seen);
        let act_q: Vec<f32> = match self.inner.last_action_qs() {
            Some(qs) if qs.len() == k => qs.to_vec(),
            _ => vec![f32::NAN; k],
        };
        let act_n: Vec<u32> = match self.inner.last_action_visits() {
            Some(ns) if ns.len() == k => ns.to_vec(),
            _ => vec![0u32; k],
        };
        let unseen_hand_count = pool.iter().filter(|(_, h)| *h).count();
        let belief_meta = obs::belief_meta(d.game, d.seat, unseen_hand_count);
        let step = Step {
            seat: d.seat,
            turn: d.game.turn,
            qkind: obs::query_index(d.query) as u8,
            obs: obs::observe(d.game, d.seat, d.query, d.candidates),
            hidden,
            hidden_ids,
            pool: pool.iter().map(|(id, _)| obs::card_id(&d.game.obj(*id).def.name)).collect(),
            pool_in_hand: pool.iter().map(|(_, h)| *h).collect(),
            chosen,
            logp: target[chosen].max(1e-8).sln(),
            qval: self.inner.last_value().unwrap_or(f32::NAN),
            target,
            act_q,
            act_n,
            belief_meta,
        };
        self.sink.lock().unwrap().push(step);
        choice
    }

    fn last_policy(&self) -> Option<&[f32]> {
        self.inner.last_policy()
    }

    fn last_value(&self) -> Option<f32> {
        self.inner.last_value()
    }

    fn last_action_qs(&self) -> Option<&[f32]> {
        self.inner.last_action_qs()
    }

    fn last_action_visits(&self) -> Option<&[u32]> {
        self.inner.last_action_visits()
    }
}

/// f32 to IEEE half, rounded to nearest (no NaN payloads)
pub fn f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xff) as i32 - 127 + 15;
    let mant = x & 0x7f_ffff;
    if v.is_nan() {
        return 0x7e00;
    }
    if exp >= 31 {
        return sign | 0x7c00;
    }
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let m = (mant | 0x80_0000) >> (1 - exp);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    let h = sign as u32 | ((exp as u32) << 10) | (mant >> 13);
    (h + ((mant >> 12) & 1)) as u16
}

/// Columnar files of recorded decisions (one set per writer): `rust/rl/data.py` reads them.
///
/// `meta.i32` has `META` ints per decision: game, seat, turn, chosen, entities, actions, hidden entities, pool
/// size, question kind, the writer's tag. `logp.f32` one float each. `glob.f16`, `ents.f16`, `ent_ids.u16`,
/// `acts.f16`, `act_ids.u16`, `target.f16`, `hid.f16`, `hid_ids.u16`, `pool.u16`, `pool_hand.u8` the rest, in
/// order. Each game's result is a line of `games.jsonl`.
pub struct Shard {
    files: Vec<std::io::BufWriter<std::fs::File>>,
    games: std::io::BufWriter<std::fs::File>,
    pub steps: usize,
}

pub const META: usize = 10;
/// The files of a shard, `<name>.<column>`: every one is its decisions (or games) one after another, so shards of
/// one build join by appending each file to the other's
pub const COLUMNS: [&str; 20] = [
    "meta.i32",
    "logp.f32",
    "glob.f16",
    "ents.f16",
    "ent_ids.u16",
    "acts.f16",
    "act_ids.u16",
    "target.f16",
    "hid.f16",
    "hid_ids.u16",
    "pool.u16",
    "pool_hand.u8",
    "qval.f16",
    "ent_rel_idx.u8",
    "ent_rel_type.u8",
    "act_ent_idx.u8",
    "act_q.f16",
    "act_n.u16",
    "belief_meta.f16",
    "games.jsonl",
];

impl Shard {
    pub fn create(dir: &str, name: &str) -> std::io::Result<Shard> {
        std::fs::create_dir_all(dir)?;
        let open = |c: &str| std::fs::File::create(format!("{dir}/{name}.{c}")).map(std::io::BufWriter::new);
        let mut files = Vec::new();
        for c in &COLUMNS[..19] {
            files.push(open(c)?);
        }
        Ok(Shard { files, games: open(COLUMNS[19])?, steps: 0 })
    }

    /// Writes one game's decisions and its result (`result[seat]`: 1 won, 0 lost, 0.5 a draw)
    pub fn game(
        &mut self,
        game: usize,
        tag: i32,
        steps: &[Step],
        result: [f32; 2],
        info: serde_json::Value,
    ) -> std::io::Result<()> {
        let h = |v: &[f32]| v.iter().flat_map(|x| f16(*x).to_le_bytes()).collect::<Vec<u8>>();
        let u = |v: &[u32]| v.iter().flat_map(|x| (*x as u16).to_le_bytes()).collect::<Vec<u8>>();
        for s in steps {
            debug_assert_eq!(s.obs.glob.len(), GLOB_F);
            debug_assert_eq!(s.obs.ents.len(), s.obs.n_ents() * ENT_F);
            debug_assert_eq!(s.obs.acts.len(), s.obs.n_acts() * ACT_F);
            let meta: [i32; META] = [
                game as i32,
                s.seat as i32,
                s.turn as i32,
                s.chosen as i32,
                s.obs.n_ents() as i32,
                s.obs.n_acts() as i32,
                s.hidden_ids.len() as i32,
                s.pool.len() as i32,
                s.qkind as i32,
                tag,
            ];
            let f = &mut self.files;
            f[0].write_all(&meta.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())?;
            f[1].write_all(&s.logp.to_le_bytes())?;
            f[2].write_all(&h(&s.obs.glob))?;
            f[3].write_all(&h(&s.obs.ents))?;
            f[4].write_all(&u(&s.obs.ent_ids))?;
            f[5].write_all(&h(&s.obs.acts))?;
            f[6].write_all(&u(&s.obs.act_ids))?;
            f[7].write_all(&h(&s.target))?;
            f[8].write_all(&h(&s.hidden))?;
            f[9].write_all(&u(&s.hidden_ids))?;
            f[10].write_all(&u(&s.pool))?;
            f[11].write_all(&s.pool_in_hand.iter().map(|b| *b as u8).collect::<Vec<u8>>())?;
            f[12].write_all(&f16(s.qval).to_le_bytes())?;
            f[13].write_all(&s.obs.ent_rel_idx)?;
            f[14].write_all(&s.obs.ent_rel_type)?;
            f[15].write_all(&s.obs.act_ent_idx)?;
            let aq = if s.act_q.len() == s.obs.n_acts() { h(&s.act_q) } else { h(&vec![f32::NAN; s.obs.n_acts()]) };
            let an: Vec<u8> = if s.act_n.len() == s.obs.n_acts() {
                s.act_n.iter().flat_map(|x| ((*x).min(u16::MAX as u32) as u16).to_le_bytes()).collect()
            } else {
                vec![0u8; s.obs.n_acts() * 2]
            };
            f[16].write_all(&aq)?;
            f[17].write_all(&an)?;
            f[18].write_all(&h(&s.belief_meta))?;
        }
        self.steps += steps.len();
        let mut line = info;
        line["game"] = game.into();
        line["result"] = serde_json::json!(result);
        line["tag"] = tag.into();
        writeln!(self.games, "{line}")
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        for f in &mut self.files {
            f.flush()?;
        }
        self.games.flush()
    }
}

/// Whether `config` plays with a network
pub fn uses_net(config: &Option<Config>) -> bool {
    config.as_ref().is_some_and(|c| {
        matches!(c.policy, PolicyKind::Net | PolicyKind::NetSearch | PolicyKind::Gumbel | PolicyKind::Ismcts)
            || c.eval >= 3
    })
}

/// The objects in a zone the seat can't see (for the leak audit)
pub fn hidden_objects(game: &Game, seat: Seat) -> Vec<ObjId> {
    let opp = game.opponent(seat);
    let mut v: Vec<ObjId> = game.player(opp).hand.clone();
    v.extend(game.player(opp).library.iter());
    v.extend(game.player(seat).library.iter());
    v.retain(|id| matches!(game.obj(*id).zone, Zone::Hand | Zone::Library));
    v
}
