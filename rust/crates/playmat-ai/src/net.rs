//! A small policy and value network, run on the CPU in plain Rust (#390 Track B). Trained by `rust/rl` (PyTorch),
//! written as a `.pmnet` file: one line of JSON naming each tensor and its shape, then the tensors as little-endian
//! f32 in that order. No ONNX runtime: the network is small enough (about 100k weights) that a plain loop answers a
//! decision in well under a millisecond, on Hetzner's ARM cores too.
//!
//! The network (`rust/rl/model.py` is the same in PyTorch):
//!
//! - each entity: its card's embedding and its features, through two ReLU layers, then the mean and the max over
//!   entities;
//! - the global features through one ReLU layer;
//! - both through two ReLU layers: the state `h`;
//! - the value: a linear layer on `h`, the logit of the seat winning;
//! - each action: its card's embedding and its features through two layers (`a`); its logit is
//!   `query(h) · a + bias(a)`;
//! - the belief (optional): an unseen card's logit of being in the opponent's hand is `belief(h) · embedding(card)`.

use crate::obs::{self, Obs, ACT_F, ENT_F, GLOB_F};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// A dense layer: `w` is `[out][in]`, flat
#[derive(Clone, Debug, Default)]
pub struct Linear {
    pub w: Vec<f32>,
    pub b: Vec<f32>,
    pub inp: usize,
    pub out: usize,
}

impl Linear {
    fn apply(&self, x: &[f32], y: &mut Vec<f32>, relu: bool) {
        debug_assert_eq!(x.len(), self.inp);
        y.clear();
        for o in 0..self.out {
            let row = &self.w[o * self.inp..(o + 1) * self.inp];
            let mut s = self.b[o];
            for (a, b) in row.iter().zip(x) {
                s += a * b;
            }
            y.push(if relu { s.max(0.0) } else { s });
        }
    }
}

#[derive(Debug, Clone)]
pub struct Net {
    pub name: String,
    pub emb: Vec<f32>,
    pub dim: usize,
    pub ent1: Linear,
    pub ent2: Linear,
    pub glob1: Linear,
    pub trunk1: Linear,
    pub trunk2: Linear,
    pub value: Linear,
    pub act1: Linear,
    pub act2: Linear,
    pub query: Linear,
    pub abias: Linear,
    pub belief: Option<Linear>,
}

/// The network's outputs for one observation
#[derive(Debug, Clone, Default)]
pub struct Output {
    /// one logit per action
    pub logits: Vec<f32>,
    /// the logit of the seat winning
    pub value: f32,
    /// the state, for the belief
    pub h: Vec<f32>,
}

impl Net {
    /// Reads a `.pmnet` file's bytes
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Net, String> {
        let nl = bytes.iter().position(|b| *b == b'\n').ok_or("no header line")?;
        let header: serde_json::Value = serde_json::from_slice(&bytes[..nl]).map_err(|e| e.to_string())?;
        let mut data = &bytes[nl + 1..];
        let mut tensors: HashMap<String, (Vec<usize>, Vec<f32>)> = HashMap::new();
        for t in header["tensors"].as_array().ok_or("no tensors")? {
            let tname = t[0].as_str().ok_or("tensor name")?.to_string();
            let shape: Vec<usize> =
                t[1].as_array().ok_or("tensor shape")?.iter().map(|v| v.as_u64().unwrap_or(0) as usize).collect();
            let n: usize = shape.iter().product();
            if data.len() < n * 4 {
                return Err(format!("{tname}: file too short"));
            }
            let v = data[..n * 4].as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect();
            data = &data[n * 4..];
            tensors.insert(tname, (shape, v));
        }
        let mut take = |base: &str| -> Result<Linear, String> {
            let (ws, w) = tensors.remove(&format!("{base}.weight")).ok_or(format!("no {base}.weight"))?;
            let (_, b) = tensors.remove(&format!("{base}.bias")).ok_or(format!("no {base}.bias"))?;
            Ok(Linear { w, b, out: ws[0], inp: ws[1] })
        };
        let ent1 = take("ent1")?;
        let ent2 = take("ent2")?;
        let glob1 = take("glob1")?;
        let trunk1 = take("trunk1")?;
        let trunk2 = take("trunk2")?;
        let value = take("value")?;
        let act1 = take("act1")?;
        let act2 = take("act2")?;
        let query = take("query")?;
        let abias = take("abias")?;
        let belief = take("belief").ok();
        let (es, emb) = tensors.remove("emb.weight").ok_or("no emb.weight")?;
        let dim = es[1];
        if ent1.inp != dim + ENT_F || glob1.inp != GLOB_F || act1.inp != dim + ACT_F {
            return Err(format!(
                "{name}: made for other features (entity {}, global {}, action {}; this build has {}, {}, {})",
                ent1.inp - dim,
                glob1.inp,
                act1.inp - dim,
                ENT_F,
                GLOB_F,
                ACT_F
            ));
        }
        Ok(Net {
            name: name.to_string(),
            emb,
            dim,
            ent1,
            ent2,
            glob1,
            trunk1,
            trunk2,
            value,
            act1,
            act2,
            query,
            abias,
            belief,
        })
    }

    fn embedding(&self, id: u32) -> &[f32] {
        let i = (id as usize).min(self.emb.len() / self.dim - 1);
        &self.emb[i * self.dim..(i + 1) * self.dim]
    }

    /// The state `h` and the value
    pub fn state(&self, o: &Obs) -> (Vec<f32>, f32) {
        let h1 = self.ent2.out;
        let mut mean = vec![0f32; h1];
        let mut max = vec![0f32; h1];
        let n = o.n_ents();
        let (mut x, mut y, mut z) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..n {
            x.clear();
            x.extend_from_slice(self.embedding(o.ent_ids[i]));
            x.extend_from_slice(&o.ents[i * ENT_F..(i + 1) * ENT_F]);
            self.ent1.apply(&x, &mut y, true);
            self.ent2.apply(&y, &mut z, true);
            for k in 0..h1 {
                mean[k] += z[k] / n as f32;
                max[k] = if i == 0 { z[k] } else { max[k].max(z[k]) };
            }
        }
        self.glob1.apply(&o.glob, &mut y, true);
        let mut t = mean;
        t.extend(max);
        t.extend_from_slice(&y);
        self.trunk1.apply(&t, &mut y, true);
        let mut h = Vec::new();
        self.trunk2.apply(&y, &mut h, true);
        self.value.apply(&h, &mut y, false);
        (h, y[0])
    }

    /// Every action's logit, and the value
    pub fn forward(&self, o: &Obs) -> Output {
        let (h, value) = self.state(o);
        let mut q = Vec::new();
        self.query.apply(&h, &mut q, false);
        let (mut x, mut y, mut a, mut b) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut logits = Vec::with_capacity(o.n_acts());
        for i in 0..o.n_acts() {
            x.clear();
            x.extend_from_slice(self.embedding(o.act_ids[i]));
            x.extend_from_slice(&o.acts[i * ACT_F..(i + 1) * ACT_F]);
            self.act1.apply(&x, &mut y, true);
            self.act2.apply(&y, &mut a, false);
            self.abias.apply(&a, &mut b, false);
            logits.push(q.iter().zip(&a).map(|(u, v)| u * v).sum::<f32>() + b[0]);
        }
        Output { logits, value, h }
    }

    /// Each card's logit of being in the opponent's hand, from the state `h` (0 for a net without a belief head)
    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        let Some(bl) = &self.belief else { return vec![0.0; cards.len()] };
        let mut q = Vec::new();
        bl.apply(h, &mut q, false);
        cards.iter().map(|c| self.embedding(*c).iter().zip(&q).map(|(a, b)| a * b).sum()).collect()
    }
}

/// Softmax of `logits / temp`
pub fn softmax(logits: &[f32], temp: f32) -> Vec<f32> {
    let t = temp.max(1e-3);
    let m = logits.iter().copied().fold(f32::MIN, f32::max);
    let e: Vec<f32> = logits.iter().map(|l| ((l - m) / t).exp()).collect();
    let s: f32 = e.iter().sum();
    e.iter().map(|v| v / s).collect()
}

static CACHE: OnceLock<Mutex<HashMap<String, Arc<Net>>>> = OnceLock::new();

/// The network at `path`, read once per process (`builtin`: the one shipped in this build, if any)
pub fn load(path: &str) -> Result<Arc<Net>, String> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(n) = cache.lock().unwrap().get(path) {
        return Ok(Arc::clone(n));
    }
    let net = if path == "builtin" {
        Net::parse("builtin", BUILTIN)?
    } else {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        Net::parse(path, &bytes)?
    };
    let net = Arc::new(net);
    cache.lock().unwrap().insert(path.to_string(), Arc::clone(&net));
    Ok(net)
}

/// The network this build ships (`rust/rl` writes it): what `net=builtin` and an unset `Config::net` load
pub const BUILTIN: &[u8] = include_bytes!("net.pmnet");

/// The net a config names (`Config::net`, else the built-in one)
pub fn of(net: &Option<String>) -> Arc<Net> {
    let path = net.as_deref().unwrap_or("builtin");
    load(path).unwrap_or_else(|e| panic!("AI network: {e}"))
}

/// The value head as a score on the fitted score's scale (a logit times 1000), `WIN`/`LOSE` for a finished game
pub fn score(net: &Net, game: &playmat_engine::state::Game, seat: playmat_engine::state::Seat) -> i64 {
    let over = crate::eval::evaluate(game, seat);
    if over >= crate::eval::WIN || over <= crate::eval::LOSE {
        return over;
    }
    let q = playmat_engine::decide::Query::Priority { playable: Vec::new() };
    let o = obs::observe(game, seat, &q, &[]);
    (net.state(&o).1 as f64 * 1000.0).clamp(-30_000.0, 30_000.0) as i64
}
