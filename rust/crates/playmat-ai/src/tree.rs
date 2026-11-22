//! `ismcts`: an open-loop information-set tree search over the network's policy (#400 R3 + Gumbel/Value-Leaf).
//!
//! Each iteration plays one belief-weighted sample of what the seat can't see (`selfplay::sample_games`).
//! - Inside the tree, nodes are keyed by the answers taken (open loop) and picked by PUCT.
//! - With `Config::gumbel_root`, depth 0 uses Gumbel-top-k + sequential halving (Danihelka et al., 2022) and records
//!   the completed-Q improved policy `softmax(logits + sigma(completed_Q))` as the training target, eliminating
//!   exploration collapse on low-prior tactical plays (instants on the opponent's turn).
//! - With `Config::leaf_blend > 0`, the expanded leaf's neural value `sigmoid(out.value)` is blended with (or at
//!   `1000`, completely replaces) the rollout evaluation, removing 2-turn rollouts for 5x-15x more tree simulations/s.

use crate::decision::{Choice, Decision, DecisionPolicy};
use crate::net::{self, Net};
use crate::search::{candidates, Config};
use crate::selfplay::{effective_temp, pick, sample_games, skipped, uniform};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, Seat, Step};
use std::sync::Arc;

/// True when no spell/ability is pending on the stack and we are not mid-combat:
/// pure value-leaf evaluation waits until the current stack/combat resolves (quiescence search).
#[inline]
fn quiescent(game: &Game) -> bool {
    game.stack.is_empty()
        && !matches!(
            game.step,
            Some(Step::DeclareAttackers | Step::DeclareBlockers | Step::FirstCombatDamage | Step::CombatDamage)
        )
}

/// Monotonic Q-value transformation for Gumbel-ISMCTS (`c_visit = 50, c_scale = 0.25`), plus terminal
/// discipline: proven wins (`q >= 0.999`) and proven losses (`q <= 0.001`) always override a low prior,
/// while non-terminal moves (`0.02..=0.98`) keep full policy-prior regularization against value noise.
fn sigma_gumbel(q: f32, most: usize) -> f32 {
    let base = (50.0 + most as f32) * 0.25 * q;
    if q >= 0.999 {
        base + 10.0
    } else if q <= 0.001 {
        base - 10.0
    } else {
        base
    }
}

/// candidates a tree node keeps, as the playouts' network does
const WIDTH: usize = 12;

/// Public+own-hand game state fingerprint for safe MCTS tree reuse across consecutive moves.
/// Reads only what `seat` can legally see (zero hidden info leak).
fn state_hash(game: &Game, seat: Seat, q: &Query) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut mix_u64 = |v: u64| {
        h ^= v;
        h = h.wrapping_mul(0x0100_0000_01b3);
    };
    mix_u64(game.turn as u64);
    mix_u64(game.step.map_or(255, |s| s as u64));
    mix_u64(game.active.map_or(255, |s| s as u64));
    mix_u64(crate::obs::query_index(q) as u64);
    mix_u64(game.players[0].life as i64 as u64);
    mix_u64(game.players[1].life as i64 as u64);
    mix_u64(game.players[0].hand.len() as u64);
    mix_u64(game.players[1].hand.len() as u64);
    mix_u64(game.stack.len() as u64);
    for s in game.stack.iter() {
        mix_u64(((s.card.0 as u64) << 8) | (s.controller as u64));
    }
    mix_u64(game.battlefield.len() as u64);
    for &id in game.battlefield.iter() {
        let o = game.obj(id);
        let flags = (o.controller as u64)
            | ((o.tapped as u64) << 2)
            | ((o.attacking.is_some() as u64) << 3)
            | ((o.blocked as u64) << 4)
            | (((o.blocking.len().min(7)) as u64) << 5)
            | (((o.damage.max(0) as u64) & 0xff) << 8);
        mix_u64(((id.0 as u64) << 16) | flags);
    }
    for &id in &game.player(seat).hand {
        mix_u64(id.0 as u64);
    }
    h
}

pub struct Kid {
    pub answer: Answer,
    pub prior: f32,
    pub n: u32,
    /// summed value from the view of the node's seat
    pub w: f64,
    pub node: Option<usize>,
}

pub struct Node {
    pub seat: Seat,
    pub hash: u64,
    pub kids: Vec<Kid>,
    pub n: u32,
}

#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

impl Tree {
    /// Extracts the reachable subtree rooted at `root_idx` into a compact new `Tree` (with `root_idx` at 0).
    fn extract_subtree(&self, root_idx: usize) -> Tree {
        let mut out = Vec::new();
        let mut map = vec![usize::MAX; self.nodes.len()];
        let mut stack = vec![root_idx];
        map[root_idx] = 0;
        out.push(Node { seat: 0, hash: 0, kids: Vec::new(), n: 0 });
        while let Some(old_i) = stack.pop() {
            let new_i = map[old_i];
            let src = &self.nodes[old_i];
            let mut kids = Vec::with_capacity(src.kids.len());
            for k in &src.kids {
                let child = k.node.map(|c| {
                    if map[c] == usize::MAX {
                        let ni = out.len();
                        map[c] = ni;
                        out.push(Node { seat: 0, hash: 0, kids: Vec::new(), n: 0 });
                        stack.push(c);
                    }
                    map[c]
                });
                kids.push(Kid { answer: k.answer.clone(), prior: k.prior, n: k.n, w: k.w, node: child });
            }
            out[new_i] = Node { seat: src.seat, hash: src.hash, kids, n: src.n };
        }
        Tree { nodes: out }
    }

    /// Finds a node within BFS depth 3 matching `(seat, hash)` and remaps it into a warm root matching `candidates`.
    fn reuse_root(&self, seat: Seat, hash: u64, candidates: &[Answer], priors: &[f32]) -> Option<Tree> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut queue = std::collections::VecDeque::from([(0usize, 0u8)]);
        let mut best: Option<(usize, u32)> = None;
        let mut best_depth: Option<u8> = None;
        while let Some((idx, depth)) = queue.pop_front() {
            if let Some(bd) = best_depth {
                if depth > bd {
                    break;
                }
            }
            let node = &self.nodes[idx];
            if node.seat == seat && node.hash == hash && !node.kids.is_empty() {
                best_depth = Some(depth);
                if best.is_none_or(|(_, bn)| node.n > bn) {
                    best = Some((idx, node.n));
                }
                continue;
            }
            if depth < 3 {
                for k in &node.kids {
                    if let Some(c) = k.node {
                        queue.push_back((c, depth + 1));
                    }
                }
            }
        }
        let (idx, _) = best?;
        let mut sub = self.extract_subtree(idx);
        let old_kids = std::mem::take(&mut sub.nodes[0].kids);
        let mut new_kids = Vec::with_capacity(candidates.len());
        let mut new_n = 0u32;
        for (ans, &p) in candidates.iter().zip(priors) {
            if let Some(old) = old_kids.iter().find(|k| &k.answer == ans) {
                // Keep deep child subtrees warm while capping stale root visit weight (<= 4)
                // so fresh CRN determinizations from the updated belief state dominate root ranking.
                let keep_n = old.n.min(4);
                let keep_w = if old.n > 0 { old.w * (keep_n as f64 / old.n as f64) } else { 0.0 };
                new_n += keep_n;
                new_kids.push(Kid { answer: ans.clone(), prior: p, n: keep_n, w: keep_w, node: old.node });
            } else {
                new_kids.push(Kid { answer: ans.clone(), prior: p, n: 0, w: 0.0, node: None });
            }
        }
        sub.nodes[0].kids = new_kids;
        sub.nodes[0].n = new_n;
        Some(sub)
    }
}

/// One iteration's way down the tree, carried by the playout (`Root::play_tree`)
pub struct Walk {
    pub tree: Tree,
    at: Option<usize>,
    path: Vec<(usize, usize)>,
    expanded: bool,
    net: Arc<Net>,
    cpuct: f32,
    pure_leaf: bool,
    root_pick: Option<usize>,
    /// Value logit (as probability 0..1) of the newly expanded leaf, with its seat
    pub leaf_value: Option<(Seat, f64)>,
    stop_now: bool,
    /// In pure-leaf mode, finish resolving an active stack or combat step before evaluating the leaf
    quiesce: bool,
    q_steps: u32,
}

impl Walk {
    pub fn new(tree: Tree, net: Arc<Net>, cpuct: f32, pure_leaf: bool, root_pick: Option<usize>) -> Walk {
        Walk {
            tree,
            at: Some(0),
            path: Vec::new(),
            expanded: false,
            net,
            cpuct,
            pure_leaf,
            root_pick,
            leaf_value: None,
            stop_now: false,
            quiesce: false,
            q_steps: 0,
        }
    }

    /// True in pure value-leaf mode (`leaf_blend == 1000`) once a new leaf has been expanded and evaluated
    pub fn should_stop(&self) -> bool {
        self.stop_now
    }

    /// The tree's answer to this question, or `None` to leave it to the playout (or stop in pure-leaf mode)
    pub fn step(&mut self, game: &Game, seat: Seat, q: &Query, quick: &Answer) -> Option<Answer> {
        if self.quiesce {
            self.q_steps += 1;
            if quiescent(game) || self.q_steps >= 24 {
                let s = net::score(&self.net, game, seat);
                let p = if s >= crate::eval::WIN {
                    1.0
                } else if s <= crate::eval::LOSE {
                    0.0
                } else {
                    crate::fitted::probability(s).clamp(0.02, 0.98)
                };
                self.leaf_value = Some((seat, p));
                self.stop_now = true;
                self.quiesce = false;
            }
            return None;
        }
        let at = self.at?;
        if skipped(q) {
            return None;
        }
        // Forced root candidate for Gumbel sequential halving
        if at == 0 {
            if let Some(i) = self.root_pick.take() {
                if i < self.tree.nodes[0].kids.len() && q.accepts(&self.tree.nodes[0].kids[i].answer) {
                    let child = match self.tree.nodes[0].kids[i].node {
                        Some(c) => c,
                        None => {
                            let c = self.tree.nodes.len();
                            self.tree.nodes.push(Node { seat, hash: 0, kids: Vec::new(), n: 0 });
                            self.tree.nodes[0].kids[i].node = Some(c);
                            c
                        }
                    };
                    self.path.push((0, i));
                    self.at = Some(child);
                    return Some(self.tree.nodes[0].kids[i].answer.clone());
                }
            }
        }
        // a question with one answer worth trying passes through without moving
        let options = candidates(game, seat, q, quick, WIDTH);
        if options.len() < 2 {
            return None;
        }
        if self.tree.nodes[at].kids.is_empty() {
            if self.expanded {
                self.at = None;
                return None;
            }
            let out = self.net.forward(&crate::obs::observe(game, seat, q, &options));
            let leaf_v = (1.0 / (1.0 + (-out.value as f64).exp())).clamp(0.02, 0.98);
            self.leaf_value = Some((seat, leaf_v));
            let p = net::softmax(&out.logits, 1.0);
            let node = &mut self.tree.nodes[at];
            node.seat = seat;
            node.hash = state_hash(game, seat, q);
            node.kids = options
                .iter()
                .cloned()
                .zip(p)
                .map(|(answer, prior)| Kid { answer, prior, n: 0, w: 0.0, node: None })
                .collect();
            self.expanded = true;
            if self.pure_leaf {
                self.at = None;
                if quiescent(game) {
                    self.stop_now = true;
                } else {
                    // Mid-stack or mid-combat: let quick answers finish resolving the stack/combat before scoring
                    self.leaf_value = None;
                    self.quiesce = true;
                }
                return None;
            }
        } else if self.tree.nodes[at].seat != seat {
            self.at = None;
            return None;
        }
        let node = &self.tree.nodes[at];
        let visited: u32 = node.kids.iter().map(|k| k.n).sum();
        let (wsum, p_vis) = node
            .kids
            .iter()
            .filter(|k| k.n > 0)
            .fold((0.0f64, 0.0f64), |(w, p), k| (w + k.w, p + k.prior as f64));
        // FPU reduction (Leela/KataGo): discount unvisited children by sqrt(visited prior mass)
        let fpu = if visited > 0 { (wsum / visited as f64 - 0.15 * p_vis.sqrt()).clamp(0.02, 0.98) } else { 0.5 };
        let sq = ((node.n + 1) as f64).sqrt();
        let score = |k: &Kid| {
            let q = if k.n > 0 { k.w / k.n as f64 } else { fpu };
            q + self.cpuct as f64 * k.prior as f64 * sq / (1.0 + k.n as f64)
        };
        let best = node
            .kids
            .iter()
            .enumerate()
            // a node reached on another sample can hold answers naming objects this one doesn't have
            .filter(|(_, k)| options.contains(&k.answer) && q.accepts(&k.answer))
            .max_by(|(i, a), (j, b)| score(a).total_cmp(&score(b)).then(j.cmp(i)))
            .map(|(i, _)| i);
        let Some(i) = best else {
            self.at = None;
            return None;
        };
        let child = match self.tree.nodes[at].kids[i].node {
            Some(c) => c,
            None => {
                let c = self.tree.nodes.len();
                self.tree.nodes.push(Node { seat, hash: 0, kids: Vec::new(), n: 0 });
                self.tree.nodes[at].kids[i].node = Some(c);
                c
            }
        };
        self.path.push((at, i));
        self.at = Some(child);
        Some(self.tree.nodes[at].kids[i].answer.clone())
    }

    /// `v` is the iteration's value for `seat` (0..1)
    fn backup(&mut self, v: f64, seat: Seat) {
        for &(n, k) in &self.path {
            let node = &mut self.tree.nodes[n];
            let mine = if node.seat == seat { v } else { 1.0 - v };
            node.n += 1;
            node.kids[k].n += 1;
            node.kids[k].w += mine;
        }
    }
}

fn eval_iteration(game: &Game, walk: &Walk, seat: Seat, base: i64, cfg: &Config, net: &Net) -> f64 {
    let over = crate::eval::evaluate(game, seat);
    if over >= crate::eval::WIN {
        return 1.0;
    }
    if over <= crate::eval::LOSE {
        return 0.0;
    }
    if cfg.leaf_blend >= 1000 {
        if let Some((ls, lv)) = walk.leaf_value {
            return if ls == seat { lv } else { 1.0 - lv };
        }
        return crate::fitted::probability(net::score(net, game, seat)).clamp(0.02, 0.98);
    }
    let roll_v = crate::eval::value_with(game, seat, base, cfg);
    if cfg.leaf_blend > 0 {
        if let Some((ls, lv)) = walk.leaf_value {
            let lv_root = if ls == seat { lv } else { 1.0 - lv };
            let a = cfg.leaf_blend as f64 / 1000.0;
            return a * lv_root + (1.0 - a) * roll_v;
        }
    }
    roll_v
}

pub struct Ismcts {
    rng: u64,
    last: Vec<f32>,
    last_val: Option<f32>,
    /// Warm subtree from the previous decision for state-hash-guarded tree reuse
    saved: Option<Tree>,
}

impl Ismcts {
    pub fn new(seed: u64) -> Ismcts {
        Ismcts { rng: seed, last: Vec::new(), last_val: None, saved: None }
    }
}

impl DecisionPolicy for Ismcts {
    fn name(&self) -> &str {
        "ismcts"
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
        let out = net.forward(&crate::obs::observe(d.game, d.seat, d.query, d.candidates));
        let prior = net::softmax(&out.logits, 1.0);
        let v_prior = 1.0 / (1.0 + (-out.value as f64).exp());
        let Some(root) = d.root else {
            self.last = prior;
            self.last_val = Some(v_prior as f32);
            let i = pick(&self.last, temp, &mut self.rng);
            return Some(Choice { answer: d.candidates[i].clone(), nodes: 0 });
        };
        let pure_leaf = cfg.leaf_blend >= 1000;
        let games = sample_games(&mut self.rng, d, root, &net, &out.h, cfg.samples.max(1));
        let stop = d.game.turn + 1 + cfg.extra_turns;
        let per = (cfg.budget / 16).clamp(50, cfg.playout_cap);
        let base = if pure_leaf { 0 } else { crate::eval::score_with(d.game, d.seat, cfg) };
        let cpuct = cfg.cpuct as f32 / 1000.0;

        if cfg.gumbel_root {
            let n = d.candidates.len();
            // Clamped log-prior prevents an over-sharpened prior (-15 logits) from drowning out a proven search win
            let logp: Vec<f32> = prior.iter().map(|&p| p.max(1e-3).ln()).collect();
            let g: Vec<f32> = (0..n)
                .map(|_| if temp == 0 { 0.0 } else { -(-(uniform(&mut self.rng).max(1e-9)).ln()).ln() })
                .collect();
            let mut alive: Vec<usize> = (0..n).collect();
            alive.sort_by(|a, b| (g[*b] + logp[*b]).total_cmp(&(g[*a] + logp[*a])));
            alive.truncate(cfg.consider.max(2).min(n));

            let cur_hash = state_hash(d.game, d.seat, d.query);
            let mut tree = self
                .saved
                .take()
                .and_then(|old| old.reuse_root(d.seat, cur_hash, d.candidates, &prior))
                .unwrap_or_else(|| {
                    let kids = d
                        .candidates
                        .iter()
                        .cloned()
                        .zip(prior.iter().copied())
                        .map(|(answer, prior)| Kid { answer, prior, n: 0, w: 0.0, node: None })
                        .collect();
                    Tree { nodes: vec![Node { seat: d.seat, hash: cur_hash, kids, n: 0 }] }
                });

            let m = alive.len();
            let rounds = (usize::BITS - (m.max(2) - 1).leading_zeros()).max(1) as usize;
            let total_iters = (cfg.iters.max(m as u32)) as usize;
            let mut nodes = 0u64;
            let mut world_idx = 0usize;
            let started = (cfg.time_ms > 0).then(std::time::Instant::now);

            'halving: for r in 0..rounds {
                let each = (total_iters / (rounds * alive.len())).max(1);
                // Common Random Numbers (CRN): compare all surviving candidates on the SAME determinization
                // per rep, eliminating paired opponent-hand variance across candidates.
                for _ in 0..each {
                    if let Some(t0) = started {
                        if world_idx > 0 && t0.elapsed().as_millis() >= cfg.time_ms as u128 {
                            break 'halving;
                        }
                    }
                    let world = &games[world_idx % games.len()];
                    world_idx += 1;
                    for &ci in &alive {
                        if !pure_leaf && nodes >= cfg.budget {
                            break 'halving;
                        }
                        if let Some(t0) = started {
                            if world_idx > 1 && t0.elapsed().as_millis() >= cfg.time_ms as u128 {
                                break 'halving;
                            }
                        }
                        let walk = Walk::new(tree, net.clone(), cpuct, pure_leaf, Some(ci));
                        let (res, mut walk) = root.play_tree(world.clone(), walk, per, stop);
                        match res {
                            Ok(o) => {
                                nodes += o.nodes.max(1);
                                let v = eval_iteration(&o.game, &walk, d.seat, base, cfg, &net);
                                walk.backup(v, d.seat);
                            }
                            Err(x) => nodes += x.max(1),
                        }
                        tree = walk.tree;
                    }
                }
                let most = tree.nodes[0].kids.iter().map(|k| k.n).max().unwrap_or(0) as usize;
                let key = |i: usize| {
                    let k = &tree.nodes[0].kids[i];
                    let q = if k.n > 0 { (k.w / k.n as f64) as f32 } else { v_prior as f32 };
                    g[i] + logp[i] + sigma_gumbel(q, most)
                };
                alive.sort_by(|a, b| key(*b).total_cmp(&key(*a)));
                if r + 1 < rounds {
                    alive.truncate(alive.len().div_ceil(2));
                }
            }
            // Final sort in case 'halving broke early on budget
            let most = tree.nodes[0].kids.iter().map(|k| k.n).max().unwrap_or(0) as usize;
            let key = |i: usize| {
                let k = &tree.nodes[0].kids[i];
                let q = if k.n > 0 { (k.w / k.n as f64) as f32 } else { v_prior as f32 };
                g[i] + logp[i] + sigma_gumbel(q, most)
            };
            alive.sort_by(|a, b| key(*b).total_cmp(&key(*a)));

            // Completed-Q improved policy target (Gumbel AlphaZero, Danihelka et al. 2022)
            let root_node = &tree.nodes[0];
            let total_n: u32 = root_node.kids.iter().map(|k| k.n).sum();
            let mut sum_p_vis = 0.0f64;
            let mut sum_p_q = 0.0f64;
            for (k, &p) in root_node.kids.iter().zip(&prior) {
                if k.n > 0 {
                    sum_p_vis += p as f64;
                    sum_p_q += (p as f64) * (k.w / k.n as f64);
                }
            }
            let v_mix = if total_n > 0 && sum_p_vis > 1e-6 {
                (v_prior + total_n as f64 * (sum_p_q / sum_p_vis)) / (1.0 + total_n as f64)
            } else {
                v_prior
            };
            let improved: Vec<f32> = (0..n)
                .map(|i| {
                    let k = &root_node.kids[i];
                    let q = if k.n > 0 { (k.w / k.n as f64) as f32 } else { v_mix as f32 };
                    logp[i] + sigma_gumbel(q, most)
                })
                .collect();
            self.last = net::softmax(&improved, 1.0);
            self.last_val = Some((v_mix as f32).clamp(0.0, 1.0));
            let pick_i = alive[0];
            self.saved = tree.nodes[0].kids[pick_i].node.map(|c| tree.extract_subtree(c));
            return Some(Choice { answer: d.candidates[pick_i].clone(), nodes });
        }

        // Classic PUCT ISMCTS (with light exploration noise at root when temp > 0)
        let root_prior: Vec<f32> = if temp > 0 && prior.len() > 1 {
            let exp_noise: Vec<f32> = (0..prior.len()).map(|_| -(uniform(&mut self.rng).max(1e-7)).ln()).collect();
            let ns: f32 = exp_noise.iter().sum::<f32>().max(1e-7);
            prior.iter().zip(&exp_noise).map(|(p, e)| 0.8 * p + 0.2 * (e / ns)).collect()
        } else {
            prior.clone()
        };
        let cur_hash = state_hash(d.game, d.seat, d.query);
        let mut tree = self
            .saved
            .take()
            .and_then(|old| old.reuse_root(d.seat, cur_hash, d.candidates, &root_prior))
            .unwrap_or_else(|| {
                let kids = d
                    .candidates
                    .iter()
                    .cloned()
                    .zip(root_prior.iter().copied())
                    .map(|(answer, prior)| Kid { answer, prior, n: 0, w: 0.0, node: None })
                    .collect();
                Tree { nodes: vec![Node { seat: d.seat, hash: cur_hash, kids, n: 0 }] }
            });
        let mut nodes = 0u64;
        let started = (cfg.time_ms > 0).then(std::time::Instant::now);
        for it in 0..cfg.iters.max(1) as usize {
            if !pure_leaf && nodes >= cfg.budget {
                break;
            }
            if let Some(t0) = started {
                if it > 0 && t0.elapsed().as_millis() >= cfg.time_ms as u128 {
                    break;
                }
            }
            let walk = Walk::new(tree, net.clone(), cpuct, pure_leaf, None);
            let (res, mut walk) = root.play_tree(games[it % games.len()].clone(), walk, per, stop);
            match res {
                Ok(o) => {
                    nodes += o.nodes.max(1);
                    let v = eval_iteration(&o.game, &walk, d.seat, base, cfg, &net);
                    walk.backup(v, d.seat);
                }
                Err(x) => nodes += x.max(1),
            }
            tree = walk.tree;
        }
        let visits: Vec<f32> = tree.nodes[0].kids.iter().map(|k| k.n as f32).collect();
        let total: f32 = visits.iter().sum();
        let wsum: f64 = tree.nodes[0].kids.iter().map(|k| k.w).sum();
        self.last = if total > 0.0 {
            self.last_val = Some(((wsum / total as f64) as f32).clamp(0.0, 1.0));
            // ties in visits go to the likelier candidate under the network
            visits.iter().zip(&prior).map(|(v, p)| (v + p * 1e-3) / (total + 1e-3)).collect()
        } else {
            self.last_val = Some(v_prior as f32);
            prior
        };
        let i = pick(&self.last, temp, &mut self.rng);
        self.saved = tree.nodes[0].kids[i].node.map(|c| tree.extract_subtree(c));
        Some(Choice { answer: d.candidates[i].clone(), nodes })
    }

    fn last_policy(&self) -> Option<&[f32]> {
        (!self.last.is_empty()).then_some(&self.last[..])
    }

    fn last_value(&self) -> Option<f32> {
        self.last_val
    }
}
