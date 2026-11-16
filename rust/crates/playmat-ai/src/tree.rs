//! `ismcts`: an open-loop information-set tree search over the network's policy (#400 R3).
//!
//! Each iteration plays one sample of what the seat can't see (`selfplay::sample_games`, weighted by the belief)
//! from the question. Where a question of either seat has been reached before along the same answers, the answer
//! is picked by PUCT (`Q + cpuct · P · √N / (1 + n)`, P the network's policy); the first new question of the
//! iteration is expanded with the network's policy, and the rest of the iteration is the ordinary playout (the quick
//! answers, or `rollout`'s network) to the end of the next turn, scored by `eval`. Nodes are keyed by the answers
//! taken, not by the game (open loop), so the samples share one tree. The answer is the most visited candidate.

use crate::decision::{Choice, Decision, DecisionPolicy};
use crate::net::{self, Net};
use crate::search::candidates;
use crate::selfplay::{pick, sample_games, skipped};
use playmat_engine::decide::{Answer, Query};
use playmat_engine::state::{Game, Seat};
use std::sync::Arc;

/// candidates a tree node keeps, as the playouts' network does
const WIDTH: usize = 12;
/// at most this many iterations, whatever the budget
const MAX_ITERS: usize = 256;

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
    pub kids: Vec<Kid>,
    pub n: u32,
}

#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

/// One iteration's way down the tree, carried by the playout (`Root::play_tree`)
pub struct Walk {
    pub tree: Tree,
    at: Option<usize>,
    path: Vec<(usize, usize)>,
    expanded: bool,
    net: Arc<Net>,
    cpuct: f32,
}

impl Walk {
    pub fn new(tree: Tree, net: Arc<Net>, cpuct: f32) -> Walk {
        Walk { tree, at: Some(0), path: Vec::new(), expanded: false, net, cpuct }
    }

    /// The tree's answer to this question, or `None` to leave it to the playout
    pub fn step(&mut self, game: &Game, seat: Seat, q: &Query, quick: &Answer) -> Option<Answer> {
        let at = self.at?;
        if skipped(q) {
            return None;
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
            let p = net::softmax(&out.logits, 1.0);
            let node = &mut self.tree.nodes[at];
            node.seat = seat;
            node.kids =
                options.into_iter().zip(p).map(|(answer, prior)| Kid { answer, prior, n: 0, w: 0.0, node: None }).collect();
            self.expanded = true;
        } else if self.tree.nodes[at].seat != seat {
            self.at = None;
            return None;
        }
        let node = &self.tree.nodes[at];
        let visited: u32 = node.kids.iter().map(|k| k.n).sum();
        let wsum: f64 = node.kids.iter().map(|k| k.w).sum();
        // an unvisited answer starts at the node's mean so far (a coin before anything is known)
        let fpu = if visited > 0 { wsum / visited as f64 } else { 0.5 };
        let sq = ((node.n + 1) as f64).sqrt();
        let score = |k: &Kid| {
            let q = if k.n > 0 { k.w / k.n as f64 } else { fpu };
            q + self.cpuct as f64 * k.prior as f64 * sq / (1.0 + k.n as f64)
        };
        let best = node
            .kids
            .iter()
            .enumerate()
            .filter(|(_, k)| q.accepts(&k.answer))
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
                self.tree.nodes.push(Node { seat, kids: Vec::new(), n: 0 });
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

pub struct Ismcts {
    rng: u64,
    last: Vec<f32>,
}

impl Ismcts {
    pub fn new(seed: u64) -> Ismcts {
        Ismcts { rng: seed, last: Vec::new() }
    }
}

impl DecisionPolicy for Ismcts {
    fn name(&self) -> &str {
        "ismcts"
    }

    fn decide(&mut self, d: &Decision) -> Option<Choice> {
        self.last.clear();
        if skipped(d.query) {
            return None;
        }
        let cfg = d.config;
        let net = net::of(&cfg.net);
        let out = net.forward(&crate::obs::observe(d.game, d.seat, d.query, d.candidates));
        let prior = net::softmax(&out.logits, 1.0);
        let Some(root) = d.root else {
            self.last = prior;
            let i = pick(&self.last, cfg.temp, &mut self.rng);
            return Some(Choice { answer: d.candidates[i].clone(), nodes: 0 });
        };
        let games = sample_games(&mut self.rng, d, root, &net, &out.h, cfg.samples.max(1));
        let stop = d.game.turn + 1 + cfg.extra_turns;
        // each iteration's playout gets what netsearch gives each of its playouts, so the budgets compare
        let per = (cfg.budget / 16).clamp(50, cfg.playout_cap);
        let base = crate::eval::score_with(d.game, d.seat, cfg);
        let kids = d
            .candidates
            .iter()
            .cloned()
            .zip(prior.iter().copied())
            .map(|(answer, prior)| Kid { answer, prior, n: 0, w: 0.0, node: None })
            .collect();
        let mut tree = Tree { nodes: vec![Node { seat: d.seat, kids, n: 0 }] };
        let cpuct = cfg.cpuct as f32 / 1000.0;
        let mut nodes = 0u64;
        for it in 0..MAX_ITERS {
            if nodes >= cfg.budget {
                break;
            }
            let walk = Walk::new(tree, net.clone(), cpuct);
            let (res, mut walk) = root.play_tree(games[it % games.len()].clone(), walk, per, stop);
            match res {
                Ok(o) => {
                    nodes += o.nodes.max(1);
                    walk.backup(crate::eval::value_with(&o.game, d.seat, base, cfg), d.seat);
                }
                Err(x) => nodes += x.max(1),
            }
            tree = walk.tree;
        }
        let visits: Vec<f32> = tree.nodes[0].kids.iter().map(|k| k.n as f32).collect();
        let total: f32 = visits.iter().sum();
        self.last = if total > 0.0 {
            // ties in visits go to the likelier candidate under the network
            visits.iter().zip(&prior).map(|(v, p)| (v + p * 1e-3) / (total + 1e-3)).collect()
        } else {
            prior
        };
        let i = pick(&self.last, cfg.temp, &mut self.rng);
        Some(Choice { answer: d.candidates[i].clone(), nodes })
    }

    fn last_policy(&self) -> Option<&[f32]> {
        (!self.last.is_empty()).then_some(&self.last[..])
    }
}
