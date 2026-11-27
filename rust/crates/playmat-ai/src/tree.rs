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
use crate::fm::Fm;
use crate::net::{self, Net};
use crate::search::{candidates, Config};
use crate::selfplay::{effective_temp, pick, sample_games_weighted, skipped, uniform};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, Seat, Step};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const CACHE_SHARDS: usize = 16;
const CACHE_MAX_PER_SHARD: usize = 512;

/// Sharded thread-safe memoization cache for pure neural network evaluations (`Obs -> (priors, leaf_v)`).
/// Eliminates redundant forward passes across determinized worlds (e.g., identical our-seat nodes across all
/// `samples` worlds in root-parallel/multi-tree search, transpositions, and consecutive priority moves).
/// What the cache keeps for an observation's hash: the priors and the leaf value
type Shard = Mutex<HashMap<(u64, u64), (Arc<[f32]>, f64)>>;

pub struct EvalCache {
    shards: Vec<Shard>,
}

impl Default for EvalCache {
    fn default() -> EvalCache {
        EvalCache::new()
    }
}

impl EvalCache {
    pub fn new() -> EvalCache {
        EvalCache { shards: (0..CACHE_SHARDS).map(|_| Mutex::new(HashMap::with_capacity(128))).collect() }
    }

    pub fn clear(&self) {
        for s in &self.shards {
            s.lock().unwrap().clear();
        }
    }

    fn get_cached(&self, key: (u64, u64)) -> Option<(Arc<[f32]>, f64)> {
        let si = (key.0 as usize) & (CACHE_SHARDS - 1);
        let map = self.shards[si].lock().unwrap();
        map.get(&key).map(|(p, v)| (Arc::clone(p), *v))
    }

    fn insert_cached(&self, key: (u64, u64), p: Arc<[f32]>, leaf_v: f64) {
        let si = (key.0 as usize) & (CACHE_SHARDS - 1);
        let mut map = self.shards[si].lock().unwrap();
        if map.len() >= CACHE_MAX_PER_SHARD {
            map.clear();
        }
        map.insert(key, (p, leaf_v));
    }

    fn forward_priors(&self, net: &Net, obs: &crate::obs::Obs) -> (Arc<[f32]>, f64) {
        let key = obs.hash128();
        if let Some(hit) = self.get_cached(key) {
            return hit;
        }
        let out = net.forward(obs);
        let leaf_v = (1.0 / (1.0 + (-out.value as f64).sexp())).clamp(0.02, 0.98);
        let p: Arc<[f32]> = Arc::from(net::softmax(&out.logits, 1.0));
        self.insert_cached(key, Arc::clone(&p), leaf_v);
        (p, leaf_v)
    }

    fn quiesce_value(&self, net: &Net, game: &Game, seat: Seat) -> f64 {
        let over = crate::eval::evaluate(game, seat);
        if over >= crate::eval::WIN {
            return 1.0;
        }
        if over <= crate::eval::LOSE {
            return 0.0;
        }
        let q = Query::Priority { playable: Vec::new() };
        let obs = crate::obs::observe(game, seat, &q, &[]);
        let key = obs.hash128();
        let si = (key.0 as usize) & (CACHE_SHARDS - 1);
        {
            let map = self.shards[si].lock().unwrap();
            if let Some((_, v)) = map.get(&key) {
                return *v;
            }
        }
        let logit = net.state(&obs).1 as f64;
        let p = (1.0 / (1.0 + (-logit).sexp())).clamp(0.02, 0.98);
        let mut map = self.shards[si].lock().unwrap();
        if map.len() >= CACHE_MAX_PER_SHARD {
            map.clear();
        }
        map.insert(key, (Arc::from([]), p));
        p
    }
}

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

/// Monotonic Q-value transformation for Gumbel-ISMCTS (`c_visit = 50`, configurable `c_scale`), plus terminal
/// discipline: proven wins (`q >= 0.999`) and proven losses (`q <= 0.001`) always override a low prior,
/// while non-terminal moves (`0.02..=0.98`) keep policy-prior regularization against value noise.
/// Aggregates multi-tree per-world Q values for candidate `i`:
/// - `cfr_root == 0`: standard belief-weighted expectation E_w[Q(w, a)]
/// - `cfr_root > 0`: blends expectation with entropic counterfactual certainty equivalent
///   `-(1/beta) * ln E_w[exp(-beta * Q(w, a))]` (beta = 3.0, alpha = cfr_root / 1000.0), penalizing
///   lines that suffer catastrophic loss in plausible opponent hands while staying exact when worlds agree.
fn aggregate_world_q(trees: &[Tree], weights: &[f64], i: usize, cfr_root: u32, fallback: f32, solver: bool) -> f32 {
    let mut nw = 0.0f64;
    let mut sw = 0.0f64;
    for (tr, &wt) in trees.iter().zip(weights) {
        let k = &tr.nodes[0].kids[i];
        if let Some(eq) = k.eff_q(solver) {
            let cnt = k.n.max(1) as f64;
            nw += wt * cnt;
            sw += wt * cnt * eq;
        }
    }
    if nw <= 0.0 {
        return fallback;
    }
    let q_mean = sw / nw;
    if cfr_root == 0 || trees.len() <= 1 {
        return q_mean as f32;
    }
    let alpha = (cfr_root as f64 / 1000.0).clamp(0.0, 0.5);
    let beta = 3.0f64;
    let mut wt_sum = 0.0f64;
    let mut exp_sum = 0.0f64;
    for (tr, &wt) in trees.iter().zip(weights) {
        let k = &tr.nodes[0].kids[i];
        if k.n > 0 {
            let qw = k.eff_q(solver).unwrap_or(k.w / k.n as f64);
            wt_sum += wt;
            exp_sum += wt * (-beta * qw).sexp();
        }
    }
    if wt_sum <= 0.0 || exp_sum <= 0.0 {
        return q_mean as f32;
    }
    let q_ent = -(exp_sum / wt_sum).sln() / beta;
    ((1.0 - alpha) * q_mean + alpha * q_ent).clamp(0.0, 1.0) as f32
}

/// Phase 3 RM+ / saddle-point belief tilting: KL-regularized worst-case weights over determinized worlds.
fn tilted_world_weights(trees: &[Tree], base_weights: &[f64], alive: &[usize], prior: &[f32]) -> Vec<f64> {
    if trees.len() <= 1 || alive.is_empty() {
        return base_weights.to_vec();
    }
    let eta = 1.25f64;
    let mut tilted = Vec::with_capacity(trees.len());
    for (tr, &bw) in trees.iter().zip(base_weights) {
        let mut psum = 0.0f64;
        let mut vsum = 0.0f64;
        for &a in alive {
            let k = &tr.nodes[0].kids[a];
            if k.n > 0 {
                let pa = (prior[a] as f64).max(1e-3);
                psum += pa;
                vsum += pa * (k.w / k.n as f64);
            }
        }
        let mult = if psum > 0.0 {
            let vw = vsum / psum;
            (-eta * (vw - 0.5)).sexp().clamp(0.5, 2.0)
        } else {
            1.0
        };
        tilted.push(bw * mult);
    }
    let bsum: f64 = base_weights.iter().sum::<f64>().max(1e-9);
    let tsum: f64 = tilted.iter().sum::<f64>().max(1e-9);
    tilted.iter().map(|w| w * (bsum / tsum)).collect()
}

fn sigma_gumbel(q: f32, most: usize, c_scale: f32) -> f32 {
    let base = (50.0 + most as f32) * c_scale * q;
    if q > 0.9801 {
        base + 500.0 * (q - 0.98)
    } else if q < 0.0199 {
        base - 500.0 * (0.02 - q)
    } else {
        base
    }
}

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
    /// MCTS-Solver proven outcome from node's seat (+1 win, -1 loss, 0 unknown)
    pub proven: i8,
    pub proven_dt: u8,
}

impl Kid {
    #[inline]
    fn new(answer: Answer, prior: f32) -> Kid {
        Kid { answer, prior, n: 0, w: 0.0, node: None, proven: 0, proven_dt: 0 }
    }

    #[inline]
    fn eff_q(&self, solver: bool) -> Option<f64> {
        if solver && self.proven != 0 {
            let dt = self.proven_dt.min(4) as f64;
            return Some(if self.proven > 0 { 1.0 - 0.004 * dt } else { 0.004 * dt });
        }
        if self.n > 0 { Some(self.w / self.n as f64) } else { None }
    }
}

pub struct Node {
    pub seat: Seat,
    pub hash: u64,
    pub kids: Vec<Kid>,
    pub n: u32,
    pub v_eval: f32,
    pub proven: i8,
    pub proven_dt: u8,
}

impl Node {
    #[inline]
    fn empty(seat: Seat) -> Node {
        Node { seat, hash: 0, kids: Vec::new(), n: 0, v_eval: 0.5, proven: 0, proven_dt: 0 }
    }
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
        out.push(Node::empty(0));
        while let Some(old_i) = stack.pop() {
            let new_i = map[old_i];
            let src = &self.nodes[old_i];
            let mut kids = Vec::with_capacity(src.kids.len());
            for k in &src.kids {
                let child = k.node.map(|c| {
                    if map[c] == usize::MAX {
                        let ni = out.len();
                        map[c] = ni;
                        out.push(Node::empty(0));
                        stack.push(c);
                    }
                    map[c]
                });
                kids.push(Kid { answer: k.answer.clone(), prior: k.prior, n: k.n, w: k.w, node: child, proven: k.proven, proven_dt: k.proven_dt });
            }
            out[new_i] = Node { seat: src.seat, hash: src.hash, kids, n: src.n, v_eval: src.v_eval, proven: src.proven, proven_dt: src.proven_dt };
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
                new_kids.push(Kid { answer: ans.clone(), prior: p, n: keep_n, w: keep_w, node: old.node, proven: 0, proven_dt: 0 });
            } else {
                new_kids.push(Kid::new(ans.clone(), p));
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
    cache: Arc<EvalCache>,
    cpuct: f32,
    pure_leaf: bool,
    quiesce_net: bool,
    width: usize,
    root_pick: Option<usize>,
    /// Value logit (as probability 0..1) of the newly expanded leaf, with its seat
    pub leaf_value: Option<(Seat, f64)>,
    stop_now: bool,
    /// In pure-leaf mode, finish resolving an active stack or combat step before evaluating the leaf
    quiesce: bool,
    q_steps: u32,
    /// Phase 3 Virtual-Loss batching: defer cache-miss leaf evaluations into a single batched GPU/CPU forward pass
    defer_eval: bool,
    vloss_applied: bool,
    pending_expand: Option<(usize, Seat, u64, Vec<Answer>, crate::obs::Obs)>,
    pending_quiesce: Option<(Seat, crate::obs::Obs)>,
    solver: bool,
    fpu_vnode: bool,
    cpuct_dyn: bool,
    /// True once the walk exits the minimax tree (e.g. quiesce / quick-answer rollout);
    /// only in-tree terminal states (!left_tree) are sound for exact MCTS-Solver proofs.
    left_tree: bool,
}

impl Walk {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tree: Tree,
        net: Arc<Net>,
        cache: Arc<EvalCache>,
        cpuct: f32,
        pure_leaf: bool,
        quiesce_net: bool,
        width: usize,
        root_pick: Option<usize>,
    ) -> Walk {
        Walk {
            tree,
            at: Some(0),
            path: Vec::new(),
            expanded: false,
            net,
            cache,
            cpuct,
            pure_leaf,
            quiesce_net,
            width: width.max(2),
            root_pick,
            leaf_value: None,
            stop_now: false,
            quiesce: false,
            q_steps: 0,
            defer_eval: false,
            vloss_applied: false,
            pending_expand: None,
            pending_quiesce: None,
            solver: false,
            fpu_vnode: false,
            cpuct_dyn: false,
            left_tree: false,
        }
    }

    fn with_opts(mut self, cfg: &Config) -> Self {
        self.solver = cfg.solver;
        self.fpu_vnode = cfg.fpu_vnode;
        self.cpuct_dyn = cfg.cpuct_dyn;
        self
    }

    fn apply_vloss(&mut self) {
        if self.vloss_applied {
            return;
        }
        for &(n, k) in &self.path {
            self.tree.nodes[n].n += 1;
            self.tree.nodes[n].kids[k].n += 1;
        }
        self.vloss_applied = true;
    }

    /// True in pure value-leaf mode (`leaf_blend == 1000`) once a new leaf has been expanded and evaluated
    pub fn should_stop(&self) -> bool {
        self.stop_now
    }

    /// The tree's answer to this question, or `None` to leave it to the playout (or stop in pure-leaf mode)
    pub fn step(&mut self, game: &Game, seat: Seat, q: &Query, quick: &Answer) -> Option<Answer> {
        if self.quiesce {
            self.q_steps += 1;
            let cap = if self.quiesce_net { 16 } else { 24 };
            if quiescent(game) || self.q_steps >= cap {
                if self.defer_eval {
                    let over = crate::eval::evaluate(game, seat);
                    if over < crate::eval::WIN && over > crate::eval::LOSE {
                        let q_dummy = Query::Priority { playable: Vec::new() };
                        let obs = crate::obs::observe(game, seat, &q_dummy, &[]);
                        if let Some((_, v)) = self.cache.get_cached(obs.hash128()) {
                            self.leaf_value = Some((seat, v));
                        } else {
                            self.pending_quiesce = Some((seat, obs));
                        }
                    }
                } else {
                    let p = self.cache.quiesce_value(&self.net, game, seat);
                    self.leaf_value = Some((seat, p));
                }
                self.stop_now = true;
                self.quiesce = false;
                return None;
            }
            self.left_tree = true;
            if self.quiesce_net && !skipped(q) {
                let options = candidates(game, seat, q, quick, self.width);
                if options.len() >= 2 {
                    let obs = crate::obs::observe(game, seat, q, &options);
                    let (p, _) = self.cache.forward_priors(&self.net, &obs);
                    if let Some((best_i, _)) = p.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)) {
                        return Some(options[best_i].clone());
                    }
                }
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
                            self.tree.nodes.push(Node::empty(seat));
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
        let options = candidates(game, seat, q, quick, self.width);
        if options.len() < 2 {
            return options.first().filter(|a| *a != quick).cloned();
        }
        if self.tree.nodes[at].kids.is_empty() {
            if self.expanded {
                self.at = None;
                self.left_tree = true;
                return None;
            }
            let obs = crate::obs::observe(game, seat, q, &options);
            let h_state = state_hash(game, seat, q);
            if self.defer_eval && self.pure_leaf {
                if let Some((p, leaf_v)) = self.cache.get_cached(obs.hash128()) {
                    self.leaf_value = Some((seat, leaf_v));
                    let node = &mut self.tree.nodes[at];
                    node.seat = seat;
                    node.hash = h_state;
                    node.v_eval = leaf_v as f32;
                    node.kids = options
                        .iter()
                        .cloned()
                        .zip(p.iter().copied())
                        .map(|(answer, prior)| Kid::new(answer, prior))
                        .collect();
                } else {
                    self.pending_expand = Some((at, seat, h_state, options, obs));
                }
                self.expanded = true;
                self.at = None;
                if quiescent(game) {
                    self.stop_now = true;
                } else {
                    self.leaf_value = None;
                    self.quiesce = true;
                    self.left_tree = true;
                }
                return None;
            }
            let (p, leaf_v) = self.cache.forward_priors(&self.net, &obs);
            self.leaf_value = Some((seat, leaf_v));
            let node = &mut self.tree.nodes[at];
            node.seat = seat;
            node.hash = h_state;
            node.v_eval = leaf_v as f32;
            node.kids = options
                .iter()
                .cloned()
                .zip(p.iter().copied())
                .map(|(answer, prior)| Kid::new(answer, prior))
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
                    self.left_tree = true;
                }
                return None;
            }
        } else if self.tree.nodes[at].seat != seat {
            self.at = None;
            self.left_tree = true;
            return None;
        }
        let node = &self.tree.nodes[at];
        let visited: u32 = node.kids.iter().map(|k| k.n).sum();
        let (wsum, p_vis) =
            node.kids.iter().filter(|k| k.n > 0).fold((0.0f64, 0.0f64), |(w, p), k| (w + k.w, p + k.prior as f64));
        // FPU reduction (Leela/KataGo): optionally anchor to node.v_eval instead of visited-kid average
        let fpu = if self.fpu_vnode {
            (node.v_eval as f64 - 0.15 * p_vis.sqrt()).clamp(0.02, 0.98)
        } else if visited > 0 {
            (wsum / visited as f64 - 0.15 * p_vis.sqrt()).clamp(0.02, 0.98)
        } else {
            0.5
        };
        let eff_cpuct = if self.cpuct_dyn {
            self.cpuct as f64 * (1.0 + ((node.n as f64 + 197.0) / 196.0).sln())
        } else {
            self.cpuct as f64
        };
        let sq = ((node.n + 1) as f64).sqrt();
        let score = |k: &Kid| {
            let q = k.eff_q(self.solver).unwrap_or(fpu);
            q + eff_cpuct * k.prior as f64 * sq / (1.0 + k.n as f64)
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
            self.left_tree = true;
            return None;
        };
        let child = match self.tree.nodes[at].kids[i].node {
            Some(c) => c,
            None => {
                let c = self.tree.nodes.len();
                self.tree.nodes.push(Node::empty(seat));
                self.tree.nodes[at].kids[i].node = Some(c);
                c
            }
        };
        self.path.push((at, i));
        self.at = Some(child);
        Some(self.tree.nodes[at].kids[i].answer.clone())
    }

    /// `v` is the iteration's value for `seat` (0..1), with optional exact terminal proven outcome.
    fn backup(&mut self, v: f64, seat: Seat, term_proven: i8, term_dt: u8) {
        backup_path(&mut self.tree.nodes, &self.path, v, seat, term_proven, term_dt, self.solver);
    }
}

fn backup_path(
    nodes: &mut [Node],
    path: &[(usize, usize)],
    v: f64,
    seat: Seat,
    term_proven: i8,
    term_dt: u8,
    solver: bool,
) {
    let mut cur_p = if solver { term_proven } else { 0 };
    let mut cur_dt = term_dt;
    let mut cur_seat = seat;
    for &(n, k) in path.iter().rev() {
        let node = &mut nodes[n];
        let mine = if node.seat == seat { v } else { 1.0 - v };
        node.n += 1;
        node.kids[k].n += 1;
        node.kids[k].w += mine;
        if cur_p != 0 {
            let kp = if node.seat == cur_seat { cur_p } else { -cur_p };
            let kid = &mut node.kids[k];
            if kid.proven == 0 {
                kid.proven = kp;
                kid.proven_dt = cur_dt;
            } else if kid.proven == kp {
                if kp > 0 {
                    kid.proven_dt = kid.proven_dt.min(cur_dt);
                } else {
                    kid.proven_dt = kid.proven_dt.max(cur_dt);
                }
            }
            // Propagate exact minimax bounds upward: ANY winning kid => winning node; ALL losing kids => losing node
            let mut best_win: Option<u8> = None;
            let mut all_lose = !node.kids.is_empty();
            let mut worst_lose: u8 = 0;
            for kk in &node.kids {
                if kk.proven > 0 {
                    best_win = Some(best_win.map_or(kk.proven_dt, |b| b.min(kk.proven_dt)));
                    all_lose = false;
                } else if kk.proven < 0 {
                    worst_lose = worst_lose.max(kk.proven_dt);
                } else {
                    all_lose = false;
                }
            }
            if let Some(wdt) = best_win {
                node.proven = 1;
                node.proven_dt = wdt;
                cur_p = 1;
                cur_dt = wdt;
                cur_seat = node.seat;
            } else if all_lose {
                node.proven = -1;
                node.proven_dt = worst_lose;
                cur_p = -1;
                cur_dt = worst_lose;
                cur_seat = node.seat;
            } else {
                cur_p = 0;
            }
        }
    }
}

fn eval_iteration(game: &Game, walk: &Walk, seat: Seat, root_turn: u32, base: i64, cfg: &Config, net: &Net) -> (f64, i8, u8) {
    let over = crate::eval::evaluate(game, seat);
    // Fastest-win / longest-survival discipline: immediate lethal (1.000) beats slow wins (0.996..0.984)
    // while staying strictly separated from non-terminal clamped leaves [0.020, 0.980].
    let dt_u = game.turn.saturating_sub(root_turn).min(4) as u8;
    let dt = dt_u as f64;
    if over >= crate::eval::WIN {
        let p = if !walk.left_tree { 1 } else { 0 };
        return (1.0 - 0.004 * dt, p, dt_u);
    }
    if over <= crate::eval::LOSE {
        let p = if !walk.left_tree { -1 } else { 0 };
        return (0.004 * dt, p, dt_u);
    }
    if cfg.leaf_blend >= 1000 {
        let v = if let Some((ls, lv)) = walk.leaf_value {
            if ls == seat { lv } else { 1.0 - lv }
        } else {
            walk.cache.quiesce_value(net, game, seat)
        };
        return (v, 0, 0);
    }
    let roll_v = crate::eval::value_with(game, seat, base, cfg);
    if cfg.leaf_blend > 0 {
        if let Some((ls, lv)) = walk.leaf_value {
            let lv_root = if ls == seat { lv } else { 1.0 - lv };
            let a = cfg.leaf_blend as f64 / 1000.0;
            return (a * lv_root + (1.0 - a) * roll_v, 0, 0);
        }
    }
    (roll_v, 0, 0)
}

/// Runs one synchronous Virtual-Loss wave across `trees` (up to `picks.len()` walks per tree), coalescing ALL
/// cache-miss leaf observations across every walk and every world into a SINGLE atomic `net.forward_batch(&obs)` call.
#[allow(clippy::too_many_arguments)]
fn run_vloss_wave(
    trees: &mut [Tree],
    games: &[Game],
    picks: &[usize],
    root: &crate::decision::Root,
    net: &Arc<Net>,
    cache: &Arc<EvalCache>,
    seat: Seat,
    root_turn: u32,
    per: u64,
    stop: u32,
    cpuct: f32,
    tree_width: usize,
    cfg: &Config,
) -> u64 {
    /// A leaf to expand once its batch is evaluated: node, seat, hash, options, the batch index and the cache key
    type Expand = (usize, Seat, u64, Vec<Answer>, usize, (u64, u64));
    struct PendingWalk {
        ti: usize,
        path: Vec<(usize, usize)>,
        end_game: Option<Game>,
        leaf_value: Option<(Seat, f64)>,
        expand_idx: Option<Expand>,
        quiesce_idx: Option<(Seat, usize, (u64, u64))>,
        left_tree: bool,
    }

    let mut obs_batch: Vec<crate::obs::Obs> = Vec::new();
    let mut pending: Vec<PendingWalk> = Vec::with_capacity(trees.len() * picks.len());
    let mut nodes = 0u64;

    for (ti, (tr, world)) in trees.iter_mut().zip(games.iter()).enumerate() {
        let mut cur_tree = std::mem::take(tr);
        for &ci in picks {
            let mut walk =
                Walk::new(cur_tree, Arc::clone(net), Arc::clone(cache), cpuct, true, false, tree_width, Some(ci))
                    .with_opts(cfg);
            walk.defer_eval = true;
            let (res, mut walk) = root.play_tree(world.clone(), walk, per, stop);
            let end_game = match res {
                Ok(o) => {
                    nodes += o.nodes.max(1);
                    Some(o.game)
                }
                Err(x) => {
                    nodes += x.max(1);
                    None
                }
            };
            walk.apply_vloss();
            let expand_idx = walk.pending_expand.take().map(|(at, es, eh, opts, o)| {
                let hk = o.hash128();
                let idx = obs_batch.len();
                obs_batch.push(o);
                (at, es, eh, opts, idx, hk)
            });
            let quiesce_idx = walk.pending_quiesce.take().map(|(qs, o)| {
                let hk = o.hash128();
                let idx = obs_batch.len();
                obs_batch.push(o);
                (qs, idx, hk)
            });
            pending.push(PendingWalk {
                ti,
                path: std::mem::take(&mut walk.path),
                end_game,
                leaf_value: walk.leaf_value,
                expand_idx,
                quiesce_idx,
                left_tree: walk.left_tree,
            });
            cur_tree = walk.tree;
        }
        *tr = cur_tree;
    }

    let outputs = net.forward_batch(&obs_batch);
    let mut eval_priors: Vec<(Arc<[f32]>, f64)> = Vec::with_capacity(outputs.len());
    for out in &outputs {
        let lv = (1.0 / (1.0 + (-out.value as f64).sexp())).clamp(0.02, 0.98);
        let p: Arc<[f32]> = Arc::from(net::softmax(&out.logits, 1.0));
        eval_priors.push((p, lv));
    }

    for pw in pending {
        let tr = &mut trees[pw.ti];
        for &(n, k) in &pw.path {
            tr.nodes[n].n = tr.nodes[n].n.saturating_sub(1);
            tr.nodes[n].kids[k].n = tr.nodes[n].kids[k].n.saturating_sub(1);
        }
        let mut leaf_val = pw.leaf_value;
        if let Some((at, es, eh, opts, idx, hk)) = pw.expand_idx {
            let (ref p, lv) = eval_priors[idx];
            cache.insert_cached(hk, Arc::clone(p), lv);
            if tr.nodes[at].kids.is_empty() {
                let node = &mut tr.nodes[at];
                node.seat = es;
                node.hash = eh;
                node.v_eval = lv as f32;
                node.kids = opts
                    .into_iter()
                    .zip(p.iter().copied())
                    .map(|(answer, prior)| Kid::new(answer, prior))
                    .collect();
            }
            if pw.quiesce_idx.is_none() && leaf_val.is_none() {
                leaf_val = Some((es, lv));
            }
        }
        if let Some((qs, idx, hk)) = pw.quiesce_idx {
            let (_, lv) = eval_priors[idx];
            cache.insert_cached(hk, Arc::from([]), lv);
            leaf_val = Some((qs, lv));
        }
        if let Some(ref g) = pw.end_game {
            let over = crate::eval::evaluate(g, seat);
            let dt_u = g.turn.saturating_sub(root_turn).min(4) as u8;
            let dt = dt_u as f64;
            let (v, tp, tdt) = if over >= crate::eval::WIN {
                (1.0 - 0.004 * dt, if !pw.left_tree { 1 } else { 0 }, dt_u)
            } else if over <= crate::eval::LOSE {
                (0.004 * dt, if !pw.left_tree { -1 } else { 0 }, dt_u)
            } else if let Some((ls, lv)) = leaf_val {
                (if ls == seat { lv } else { 1.0 - lv }, 0, 0)
            } else {
                (cache.quiesce_value(net, g, seat), 0, 0)
            };
            backup_path(&mut tr.nodes, &pw.path, v, seat, tp, tdt, cfg.solver);
        }
    }
    nodes
}

pub struct Ismcts {
    rng: u64,
    last: Vec<f32>,
    last_val: Option<f32>,
    last_qs: Vec<f32>,
    last_visits: Vec<u32>,
    /// Warm subtree from the previous decision for state-hash-guarded tree reuse
    saved: Option<Tree>,
    /// Exact neural-evaluation memoization cache shared across worlds and consecutive moves
    cache: Arc<EvalCache>,
    cache_net_ptr: usize,
}

impl Ismcts {
    pub fn new(seed: u64) -> Ismcts {
        Ismcts {
            rng: seed,
            last: Vec::new(),
            last_val: None,
            last_qs: Vec::new(),
            last_visits: Vec::new(),
            saved: None,
            cache: Arc::new(EvalCache::new()),
            cache_net_ptr: 0,
        }
    }
}

impl DecisionPolicy for Ismcts {
    fn name(&self) -> &str {
        "ismcts"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.last.clear();
        self.last_val = None;
        self.last_qs.clear();
        self.last_visits.clear();
        if skipped(d.query) {
            return None;
        }
        let cfg = d.config;
        let temp = effective_temp(cfg, d.game.turn);
        let net = net::of(&cfg.net);
        let net_ptr = Arc::as_ptr(&net) as usize;
        if self.cache_net_ptr != net_ptr {
            self.cache.clear();
            self.cache_net_ptr = net_ptr;
        }
        let out = net.forward(&crate::obs::observe(d.game, d.seat, d.query, d.candidates));
        let prior = net::softmax(&out.logits, 1.0);
        let v_prior = 1.0 / (1.0 + (-out.value as f64).sexp());
        let Some(root) = d.root else {
            self.last = prior;
            self.last_val = Some(v_prior as f32);
            let i = pick(&self.last, temp, &mut self.rng);
            return Some(Choice { answer: d.candidates[i].clone(), nodes: 0 });
        };
        let pure_leaf = cfg.leaf_blend >= 1000;
        let quiesce_net = cfg.quiesce_net;
        let tree_width = cfg.width.max(2);
        let weighted = sample_games_weighted(&mut self.rng, d, root, &net, &out.h, cfg.samples.max(1));
        let (mut games, weights): (Vec<Game>, Vec<f64>) = weighted.into_iter().unzip();
        let root_turn = d.game.turn;
        let stop = root_turn + 1 + cfg.extra_turns;
        let per = (cfg.budget / 16).clamp(50, cfg.playout_cap);
        let base = if pure_leaf { 0 } else { crate::eval::score_with(d.game, d.seat, cfg) };
        let cpuct = cfg.cpuct as f32 / 1000.0;

        if cfg.gumbel_root {
            let n = d.candidates.len();
            // Clamped log-prior prevents an over-sharpened prior (-15 logits) from drowning out a proven search win
            let logp: Vec<f32> = prior.iter().map(|&p| p.max(1e-3).sln()).collect();
            let sel_logp: Vec<f32> = if temp > 0 && cfg.forced_k > 0 && n > 1 {
                let eps = (cfg.forced_k as f32 / 10000.0).clamp(0.005, 0.08);
                let uni = 1.0 / n as f32;
                prior.iter().map(|&p| ((1.0 - eps) * p + eps * uni).max(1e-3).sln()).collect()
            } else {
                logp.clone()
            };
            let g: Vec<f32> = (0..n)
                .map(|_| if temp == 0 { 0.0 } else { -(-(uniform(&mut self.rng).max(1e-9)).sln()).sln() })
                .collect();
            let mut alive: Vec<usize> = (0..n).collect();
            alive.sort_by(|a, b| (g[*b] + sel_logp[*b]).total_cmp(&(g[*a] + sel_logp[*a])));
            alive.truncate(cfg.consider.max(2).min(n));

            let cur_hash = state_hash(d.game, d.seat, d.query);
            let m = alive.len();
            let rounds = (usize::BITS - (m.max(2) - 1).leading_zeros()).max(1) as usize;
            let total_iters = (cfg.iters.max(m as u32)) as usize;
            let started = (cfg.time_ms > 0).then(std::time::Instant::now);
            let c_scale = cfg.c_scale as f32 / 1000.0;

            if (cfg.par > 1 || cfg.multi_tree) && games.len() > 1 {
                // Multi-Tree Determinized Gumbel-ISMCTS (sequential when par == 1, root-parallel when par > 1):
                // Each determinized world owns an independent tree (eliminating cross-world strategy fusion),
                // aggregated by CRN at the root every halving round.
                let w_cnt = games.len();
                let n_threads = (cfg.par as usize).min(w_cnt).max(1);
                let chunk_sz = w_cnt.div_ceil(n_threads);
                let mut trees: Vec<Tree> = (0..w_cnt)
                    .map(|_| {
                        let kids = d
                            .candidates
                            .iter()
                            .cloned()
                            .zip(prior.iter().copied())
                            .map(|(answer, prior)| Kid::new(answer, prior))
                            .collect();
                        Tree { nodes: vec![Node { seat: d.seat, hash: cur_hash, kids, n: 0, v_eval: v_prior as f32, proven: 0, proven_dt: 0 }] }
                    })
                    .collect();
                let mut nodes = 0u64;
                let mut rem_iters = total_iters;
                'par_halving: for r in 0..rounds {
                    let rounds_left = rounds - r;
                    let round_budget = rem_iters / rounds_left;
                    let each_per_world = (round_budget / (alive.len() * w_cnt)).max(1);
                    rem_iters = rem_iters.saturating_sub(each_per_world * alive.len() * w_cnt);
                    // Time-aware halving: reserve equal wall-clock slices per round so the final top-2 showdown
                    // always runs, and prefer breaking between full CRN reps over breaking mid-round.
                    let round_deadline = (cfg.time_ms as u128 * (r as u128 + 1)) / rounds as u128;
                    let seat = d.seat;
                    if cfg.vloss > 1 && pure_leaf && !quiesce_net {
                        let mut schedule: Vec<usize> = Vec::with_capacity(each_per_world * alive.len());
                        for _ in 0..each_per_world {
                            schedule.extend_from_slice(&alive);
                        }
                        for chunk in schedule.chunks(cfg.vloss) {
                            if let Some(t0) = started {
                                let el = t0.elapsed().as_millis();
                                if el >= round_deadline || (r > 0 && el >= cfg.time_ms as u128) {
                                    break;
                                }
                            }
                            nodes += run_vloss_wave(
                                &mut trees,
                                &games,
                                chunk,
                                root,
                                &net,
                                &self.cache,
                                seat,
                                root_turn,
                                per,
                                stop,
                                cpuct,
                                tree_width,
                                d.config,
                            );
                        }
                    } else if n_threads <= 1 {
                        for (tree, world) in trees.iter_mut().zip(games.iter()) {
                            let mut tr = std::mem::take(tree);
                            'wrep_seq: for rep in 0..each_per_world {
                                if let Some(t0) = started {
                                    let el = t0.elapsed().as_millis();
                                    if (rep > 0 && el >= round_deadline)
                                        || ((r > 0 || rep > 0) && el >= cfg.time_ms as u128)
                                    {
                                        break 'wrep_seq;
                                    }
                                }
                                for &ci in &alive {
                                    if let Some(t0) = started {
                                        if (r > 0 || rep > 0) && t0.elapsed().as_millis() >= cfg.time_ms as u128 {
                                            break 'wrep_seq;
                                        }
                                    }
                                    let walk = Walk::new(
                                        tr,
                                        net.clone(),
                                        self.cache.clone(),
                                        cpuct,
                                        pure_leaf,
                                        quiesce_net,
                                        tree_width,
                                        Some(ci),
                                    )
                                    .with_opts(cfg);
                                    let (res, mut walk) = root.play_tree(world.clone(), walk, per, stop);
                                    match res {
                                        Ok(o) => {
                                            nodes += o.nodes.max(1);
                                            let (v, tp, tdt) = eval_iteration(&o.game, &walk, seat, root_turn, base, cfg, &net);
                                            walk.backup(v, seat, tp, tdt);
                                        }
                                        Err(x) => nodes += x.max(1),
                                    }
                                    tr = walk.tree;
                                }
                            }
                            *tree = tr;
                        }
                    } else {
                        let round_nodes = std::sync::atomic::AtomicU64::new(0);
                        let alive_ref = &alive;
                        let net_ref = &net;
                        let cache_ref = &self.cache;
                        std::thread::scope(|s| {
                            for (t_chunk, g_chunk) in trees.chunks_mut(chunk_sz).zip(games.chunks_mut(chunk_sz)) {
                                let rn = &round_nodes;
                                let root_local = root.clone();
                                s.spawn(move || {
                                    let mut local_nodes = 0u64;
                                    for (tree, world) in t_chunk.iter_mut().zip(g_chunk.iter()) {
                                        let mut tr = std::mem::take(tree);
                                        'wrep: for rep in 0..each_per_world {
                                            if let Some(t0) = started {
                                                let el = t0.elapsed().as_millis();
                                                if (rep > 0 && el >= round_deadline)
                                                    || ((r > 0 || rep > 0) && el >= cfg.time_ms as u128)
                                                {
                                                    break 'wrep;
                                                }
                                            }
                                            for &ci in alive_ref {
                                                if let Some(t0) = started {
                                                    if (r > 0 || rep > 0)
                                                        && t0.elapsed().as_millis() >= cfg.time_ms as u128
                                                    {
                                                        break 'wrep;
                                                    }
                                                }
                                                let walk = Walk::new(
                                                    tr,
                                                    net_ref.clone(),
                                                    cache_ref.clone(),
                                                    cpuct,
                                                    pure_leaf,
                                                    quiesce_net,
                                                    tree_width,
                                                    Some(ci),
                                                )
                                                .with_opts(cfg);
                                                let (res, mut walk) =
                                                    root_local.play_tree(world.clone(), walk, per, stop);
                                                match res {
                                                    Ok(o) => {
                                                        local_nodes += o.nodes.max(1);
                                                        let (v, tp, tdt) = eval_iteration(
                                                            &o.game, &walk, seat, root_turn, base, cfg, net_ref,
                                                        );
                                                        walk.backup(v, seat, tp, tdt);
                                                    }
                                                    Err(x) => local_nodes += x.max(1),
                                                }
                                                tr = walk.tree;
                                            }
                                        }
                                        *tree = tr;
                                    }
                                    rn.fetch_add(local_nodes, std::sync::atomic::Ordering::Relaxed);
                                });
                            }
                        });
                        nodes += round_nodes.load(std::sync::atomic::Ordering::Relaxed);
                    }
                    let mut agg_n = vec![0u32; n];
                    let mut agg_nw = vec![0.0f64; n];
                    let mut agg_w = vec![0.0f64; n];
                    for (tr, &wt) in trees.iter().zip(&weights) {
                        for (i, k) in tr.nodes[0].kids.iter().enumerate() {
                            agg_n[i] += k.n;
                            agg_nw[i] += wt * k.n as f64;
                            agg_w[i] += wt * k.w;
                        }
                    }
                    let eff_w = if cfg.rm_root {
                        tilted_world_weights(&trees, &weights, &alive, &prior)
                    } else {
                        weights.clone()
                    };
                    let most = agg_n.iter().copied().max().unwrap_or(0) as usize;
                    let key = |i: usize| {
                        let q = aggregate_world_q(&trees, &eff_w, i, cfg.cfr_root, v_prior as f32, cfg.solver);
                        g[i] + logp[i] + sigma_gumbel(q, most, c_scale)
                    };
                    alive.sort_by(|a, b| key(*b).total_cmp(&key(*a)));
                    if r + 1 < rounds {
                        alive.truncate(alive.len().div_ceil(2));
                    }
                    if let Some(t0) = started {
                        if t0.elapsed().as_millis() >= cfg.time_ms as u128 {
                            break 'par_halving;
                        }
                    }
                }
                let mut agg_n = vec![0u32; n];
                let mut agg_nw = vec![0.0f64; n];
                let mut agg_w = vec![0.0f64; n];
                for (tr, &wt) in trees.iter().zip(&weights) {
                    for (i, k) in tr.nodes[0].kids.iter().enumerate() {
                        agg_n[i] += k.n;
                        agg_nw[i] += wt * k.n as f64;
                        agg_w[i] += wt * k.w;
                    }
                }
                let eff_w =
                    if cfg.rm_root { tilted_world_weights(&trees, &weights, &alive, &prior) } else { weights.clone() };
                let most = agg_n.iter().copied().max().unwrap_or(0) as usize;
                let key = |i: usize| {
                    let q = aggregate_world_q(&trees, &eff_w, i, cfg.cfr_root, v_prior as f32, cfg.solver);
                    g[i] + logp[i] + sigma_gumbel(q, most, c_scale)
                };
                alive.sort_by(|a, b| key(*b).total_cmp(&key(*a)));
                let total_n: u32 = agg_n.iter().sum();
                let mut sum_p_vis = 0.0f64;
                let mut sum_p_q = 0.0f64;
                for (i, &p) in prior.iter().enumerate() {
                    if agg_nw[i] > 0.0 {
                        sum_p_vis += p as f64;
                        sum_p_q += (p as f64) * (agg_w[i] / agg_nw[i]);
                    }
                }
                let v_mix = if total_n > 0 && sum_p_vis > 1e-6 {
                    (v_prior + total_n as f64 * (sum_p_q / sum_p_vis)) / (1.0 + total_n as f64)
                } else {
                    v_prior
                };
                let improved: Vec<f32> = (0..n)
                    .map(|i| {
                        let q = aggregate_world_q(&trees, &weights, i, cfg.cfr_root, v_mix as f32, cfg.solver);
                        logp[i] + sigma_gumbel(q, most, c_scale)
                    })
                    .collect();
                if std::env::var_os("PLAYMAT_AI_TRACE_ROOT").is_some() {
                    eprintln!(
                        "ROOT turn={} {:?} seat={} v_prior={:.4} v_mix={:.4} most={}",
                        d.game.turn, d.game.step, d.seat, v_prior, v_mix, most
                    );
                    for i in 0..n {
                        let q = if agg_nw[i] > 0.0 { (agg_w[i] / agg_nw[i]) as f32 } else { f32::NAN };
                        eprintln!(
                            "  cand[{i}] {:?} prior={:.4} logp={:.3} n={} q={:.4} key={:.3}",
                            d.candidates[i],
                            prior[i],
                            logp[i],
                            agg_n[i],
                            q,
                            key(i)
                        );
                    }
                }
                self.last = net::softmax(&improved, 1.0);
                self.last_val = Some((v_mix as f32).clamp(0.0, 1.0));
                self.last_qs =
                    (0..n).map(|i| if agg_nw[i] > 0.0 { (agg_w[i] / agg_nw[i]) as f32 } else { f32::NAN }).collect();
                self.last_visits = agg_n;
                self.saved = None;
                return Some(Choice { answer: d.candidates[alive[0]].clone(), nodes });
            }

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
                        .map(|(answer, prior)| Kid::new(answer, prior))
                        .collect();
                    Tree { nodes: vec![Node { seat: d.seat, hash: cur_hash, kids, n: 0, v_eval: v_prior as f32, proven: 0, proven_dt: 0 }] }
                });

            let mut nodes = 0u64;
            let mut world_idx = 0usize;

            'halving: for r in 0..rounds {
                let each = (total_iters / (rounds * alive.len())).max(1);
                if cfg.vloss > 1 && pure_leaf && !quiesce_net {
                    let mut schedule: Vec<usize> = Vec::with_capacity(each * alive.len());
                    for _ in 0..each {
                        schedule.extend_from_slice(&alive);
                    }
                    let mut trees_one = [std::mem::take(&mut tree)];
                    for chunk in schedule.chunks(cfg.vloss) {
                        if let Some(t0) = started {
                            if world_idx > 0 && t0.elapsed().as_millis() >= cfg.time_ms as u128 {
                                break;
                            }
                        }
                        let w_slice = [games[world_idx % games.len()].clone()];
                        world_idx += 1;
                        nodes += run_vloss_wave(
                            &mut trees_one,
                            &w_slice,
                            chunk,
                            root,
                            &net,
                            &self.cache,
                            d.seat,
                            root_turn,
                            per,
                            stop,
                            cpuct,
                            tree_width,
                            d.config,
                        );
                    }
                    tree = std::mem::take(&mut trees_one[0]);
                    let most = tree.nodes[0].kids.iter().map(|k| k.n).max().unwrap_or(0) as usize;
                    let key = |i: usize| {
                        let k = &tree.nodes[0].kids[i];
                        let q = k.eff_q(cfg.solver).map(|x| x as f32).unwrap_or(v_prior as f32);
                        g[i] + logp[i] + sigma_gumbel(q, most, c_scale)
                    };
                    alive.sort_by(|a, b| key(*b).total_cmp(&key(*a)));
                    if r + 1 < rounds {
                        alive.truncate(alive.len().div_ceil(2));
                    }
                    continue 'halving;
                }
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
                        let walk = Walk::new(
                            tree,
                            net.clone(),
                            self.cache.clone(),
                            cpuct,
                            pure_leaf,
                            quiesce_net,
                            tree_width,
                            Some(ci),
                        )
                        .with_opts(cfg);
                        let (res, mut walk) = root.play_tree(world.clone(), walk, per, stop);
                        match res {
                            Ok(o) => {
                                nodes += o.nodes.max(1);
                                let (v, tp, tdt) = eval_iteration(&o.game, &walk, d.seat, root_turn, base, cfg, &net);
                                walk.backup(v, d.seat, tp, tdt);
                            }
                            Err(x) => nodes += x.max(1),
                        }
                        tree = walk.tree;
                    }
                }
                let most = tree.nodes[0].kids.iter().map(|k| k.n).max().unwrap_or(0) as usize;
                let key = |i: usize| {
                    let k = &tree.nodes[0].kids[i];
                    let q = k.eff_q(cfg.solver).map(|x| x as f32).unwrap_or(v_prior as f32);
                    g[i] + logp[i] + sigma_gumbel(q, most, c_scale)
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
                let q = k.eff_q(cfg.solver).map(|x| x as f32).unwrap_or(v_prior as f32);
                g[i] + logp[i] + sigma_gumbel(q, most, c_scale)
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
                    let q = k.eff_q(cfg.solver).map(|x| x as f32).unwrap_or(v_mix as f32);
                    logp[i] + sigma_gumbel(q, most, c_scale)
                })
                .collect();
            if std::env::var_os("PLAYMAT_AI_TRACE_ROOT").is_some() {
                eprintln!(
                    "ROOT(1T) turn={} {:?} seat={} v_prior={:.4} v_mix={:.4} most={}",
                    d.game.turn, d.game.step, d.seat, v_prior, v_mix, most
                );
                for i in 0..n {
                    let k = &root_node.kids[i];
                    let q = if k.n > 0 { (k.w / k.n as f64) as f32 } else { f32::NAN };
                    eprintln!(
                        "  cand[{i}] {:?} prior={:.4} logp={:.3} n={} q={:.4} prov=({},{}) key={:.3}",
                        d.candidates[i],
                        prior[i],
                        logp[i],
                        k.n,
                        q,
                        k.proven,
                        k.proven_dt,
                        key(i)
                    );
                }
            }
            self.last = net::softmax(&improved, 1.0);
            self.last_val = Some((v_mix as f32).clamp(0.0, 1.0));
            self.last_qs =
                root_node.kids.iter().map(|k| if k.n > 0 { (k.w / k.n as f64) as f32 } else { f32::NAN }).collect();
            self.last_visits = root_node.kids.iter().map(|k| k.n).collect();
            let pick_i = alive[0];
            self.saved = tree.nodes[0].kids[pick_i].node.map(|c| tree.extract_subtree(c));
            return Some(Choice { answer: d.candidates[pick_i].clone(), nodes });
        }

        // Classic PUCT ISMCTS (with light exploration noise at root when temp > 0)
        let root_prior: Vec<f32> = if temp > 0 && prior.len() > 1 {
            let exp_noise: Vec<f32> = (0..prior.len()).map(|_| -(uniform(&mut self.rng).max(1e-7)).sln()).collect();
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
                    .map(|(answer, prior)| Kid::new(answer, prior))
                    .collect();
                Tree { nodes: vec![Node { seat: d.seat, hash: cur_hash, kids, n: 0, v_eval: v_prior as f32, proven: 0, proven_dt: 0 }] }
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
            let walk =
                Walk::new(tree, net.clone(), self.cache.clone(), cpuct, pure_leaf, quiesce_net, tree_width, None).with_opts(cfg);
            let (res, mut walk) = root.play_tree(games[it % games.len()].clone(), walk, per, stop);
            match res {
                Ok(o) => {
                    nodes += o.nodes.max(1);
                    let (v, tp, tdt) = eval_iteration(&o.game, &walk, d.seat, root_turn, base, cfg, &net);
                    walk.backup(v, d.seat, tp, tdt);
                }
                Err(x) => nodes += x.max(1),
            }
            tree = walk.tree;
        }
        let mut visits: Vec<f32> = tree.nodes[0].kids.iter().map(|k| k.n as f32).collect();
        // KataGo policy target pruning for PUCT when temp > 0 && forced_k > 0
        if temp > 0 && cfg.forced_k > 0 && visits.len() > 1 {
            let best_i = visits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let root_n = tree.nodes[0].n.max(1) as f64;
            let best_k = &tree.nodes[0].kids[best_i];
            let best_q = best_k.eff_q(cfg.solver).unwrap_or(0.5);
            let best_puct = best_q + cpuct as f64 * best_k.prior as f64 * root_n.sqrt() / (1.0 + best_k.n as f64);
            let fk = cfg.forced_k as f64 / 100.0;
            for (i, k) in tree.nodes[0].kids.iter().enumerate() {
                if i == best_i || k.n == 0 {
                    continue;
                }
                let q = k.eff_q(cfg.solver).unwrap_or(0.5);
                let forced = (fk * k.prior as f64 * root_n).sqrt().floor() as u32;
                if q < best_q && forced > 0 {
                    let mut n_eff = k.n;
                    let floor_n = k.n.saturating_sub(forced);
                    while n_eff > floor_n {
                        let cand_puct = q + cpuct as f64 * k.prior as f64 * root_n.sqrt() / (1.0 + n_eff as f64);
                        if cand_puct < best_puct {
                            n_eff -= 1;
                        } else {
                            break;
                        }
                    }
                    visits[i] = if n_eff <= 1 && forced >= 1 { 0.0 } else { n_eff as f32 };
                }
            }
        }
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

    fn last_action_qs(&self) -> Option<&[f32]> {
        (!self.last_qs.is_empty()).then_some(&self.last_qs[..])
    }

    fn last_action_visits(&self) -> Option<&[u32]> {
        (!self.last_visits.is_empty()).then_some(&self.last_visits[..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_cache_matches_direct_forward() {
        let net = net::of(&None);
        let cache = EvalCache::new();
        let obs = crate::obs::Obs {
            glob: vec![0.5; crate::obs::GLOB_F],
            ents: vec![0.25; crate::obs::ENT_F * 2],
            ent_ids: vec![1, 2],
            ent_rel_idx: vec![crate::obs::NO_ENT, crate::obs::NO_ENT],
            ent_rel_type: vec![0, 0],
            acts: vec![0.1; crate::obs::ACT_F * 2],
            act_ids: vec![0, 3],
            act_ent_idx: vec![crate::obs::NO_ENT, 0],
        };
        let mut obs2 = obs.clone();
        obs2.glob[0] = 0.6;
        assert_ne!(obs.hash128(), obs2.hash128());

        let direct = net.forward(&obs);
        let direct_p = net::softmax(&direct.logits, 1.0);
        let direct_v = (1.0 / (1.0 + (-direct.value as f64).sexp())).clamp(0.02, 0.98);

        let (p1, v1) = cache.forward_priors(&net, &obs);
        let (p2, v2) = cache.forward_priors(&net, &obs);
        assert_eq!(&*p1, direct_p.as_slice());
        assert_eq!(&*p2, direct_p.as_slice());
        assert_eq!(v1, direct_v);
        assert_eq!(v2, direct_v);
    }

    #[test]
    fn tilted_weights_preserve_sum_and_upweight_vulnerable_worlds() {
        let mk = |q: f64| Tree {
            nodes: vec![Node {
                seat: 0,
                hash: 0,
                kids: vec![Kid { answer: Answer::Bool(false), prior: 1.0, n: 10, w: 10.0 * q, node: None, proven: 0, proven_dt: 0 }],
                n: 10,
                v_eval: q as f32,
                proven: 0,
                proven_dt: 0,
            }],
        };
        let trees = vec![mk(0.9), mk(0.1)];
        let base = vec![1.0, 1.0];
        let tw = tilted_world_weights(&trees, &base, &[0], &[1.0]);
        assert!((tw.iter().sum::<f64>() - 2.0).abs() < 1e-6);
        assert!(tw[1] > tw[0], "vulnerable world (q=0.1) should get higher saddle-point weight");
    }
}
