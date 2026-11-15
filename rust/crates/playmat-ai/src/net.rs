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
            let s = self.b[o] + dot(row, x);
            y.push(if relu { s.max(0.0) } else { s });
        }
    }
}

/// A dot product in 8 lanes: the compiler vectorizes it (one running sum can't be reordered, so it doesn't)
#[inline]
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let (a, b) = (&a[..n], &b[..n]);
    let mut acc = [0f32; 8];
    let (ac, ar) = a.as_chunks::<8>();
    let (bc, br) = b.as_chunks::<8>();
    for (x, y) in ac.iter().zip(bc) {
        for i in 0..8 {
            acc[i] += x[i] * y[i];
        }
    }
    let mut s = acc.iter().sum::<f32>();
    for (x, y) in ar.iter().zip(br) {
        s += x * y;
    }
    s
}

/// The first network (#390): entities pooled by mean and max
#[derive(Debug, Clone)]
pub struct Mlp {
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

impl Mlp {
    /// Reads a `.pmnet` file's bytes
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Mlp, String> {
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
        Ok(Mlp {
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

/// A `.pmnet` file's header and tensors by name
type Tensors = HashMap<String, (Vec<usize>, Vec<f32>)>;

fn read(bytes: &[u8]) -> Result<(serde_json::Value, Tensors), String> {
    let nl = bytes.iter().position(|b| *b == b'\n').ok_or("no header line")?;
    let header: serde_json::Value = serde_json::from_slice(&bytes[..nl]).map_err(|e| e.to_string())?;
    let mut data = &bytes[nl + 1..];
    let mut tensors = HashMap::new();
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
    Ok((header, tensors))
}

#[derive(Clone, Debug, Default)]
pub struct LayerNorm {
    pub w: Vec<f32>,
    pub b: Vec<f32>,
}

impl LayerNorm {
    fn apply(&self, x: &[f32], y: &mut Vec<f32>) {
        let n = x.len() as f32;
        let mean = x.iter().sum::<f32>() / n;
        let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / n;
        let inv = 1.0 / (var + 1e-5).sqrt();
        y.clear();
        y.extend(x.iter().zip(&self.w).zip(&self.b).map(|((v, w), b)| (v - mean) * inv * w + b));
    }
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub ln1: LayerNorm,
    pub qkv: Linear,
    pub out: Linear,
    pub ln2: LayerNorm,
    pub ff1: Linear,
    pub ff2: Linear,
}

/// The transformer network (#400 R2; `rust/rl/model.py` `Tx`): a global token and one per entity through pre-norm
/// self-attention blocks, then each candidate attending to the tokens once
#[derive(Clone, Debug, Default)]
pub struct Tx {
    pub name: String,
    pub emb: Vec<f32>,
    pub dim: usize,
    pub heads: usize,
    pub ent_in: Linear,
    pub glob_in: Linear,
    pub blocks: Vec<Block>,
    pub ln_f: LayerNorm,
    pub trunk1: Linear,
    pub value: Linear,
    pub belief: Linear,
    pub act_in: Linear,
    pub act_ln: LayerNorm,
    pub act_q: Linear,
    pub act_kv: Linear,
    pub act_o: Linear,
    pub act_ff: Linear,
    pub query: Linear,
    pub abias: Linear,
}

/// Multi-head attention: each query row over every key row (Rust has only real tokens, no padding)
fn attend(q: &[Vec<f32>], k: &[Vec<f32>], v: &[Vec<f32>], heads: usize) -> Vec<Vec<f32>> {
    let d = q.first().map_or(0, |r| r.len());
    let hd = d / heads;
    let scale = 1.0 / (hd as f32).sqrt();
    let mut out = vec![vec![0f32; d]; q.len()];
    let mut w = vec![0f32; k.len()];
    for (qi, qr) in q.iter().enumerate() {
        for h in 0..heads {
            let r = h * hd..(h + 1) * hd;
            let mut m = f32::MIN;
            for (j, kr) in k.iter().enumerate() {
                let s = dot(&qr[r.clone()], &kr[r.clone()]) * scale;
                w[j] = s;
                m = m.max(s);
            }
            let mut z = 0.0;
            for x in w.iter_mut() {
                *x = (*x - m).exp();
                z += *x;
            }
            let o = &mut out[qi][r.clone()];
            for (j, vr) in v.iter().enumerate() {
                let p = w[j] / z;
                for (oo, vv) in o.iter_mut().zip(&vr[r.clone()]) {
                    *oo += p * vv;
                }
            }
        }
    }
    out
}

fn take_linear(t: &mut Tensors, b: &str) -> Result<Linear, String> {
    let (ws, w) = t.remove(&format!("{b}.weight")).ok_or(format!("no {b}.weight"))?;
    let (_, bb) = t.remove(&format!("{b}.bias")).ok_or(format!("no {b}.bias"))?;
    Ok(Linear { w, b: bb, out: ws[0], inp: ws[1] })
}

fn take_norm(t: &mut Tensors, b: &str) -> Result<LayerNorm, String> {
    let (_, w) = t.remove(&format!("{b}.weight")).ok_or(format!("no {b}.weight"))?;
    let (_, bb) = t.remove(&format!("{b}.bias")).ok_or(format!("no {b}.bias"))?;
    Ok(LayerNorm { w, b: bb })
}

impl Tx {
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Tx, String> {
        let (header, mut t) = read(bytes)?;
        let cfg = &header["config"];
        let heads = cfg["heads"].as_u64().ok_or("no heads")? as usize;
        let layers = cfg["layers"].as_u64().ok_or("no layers")? as usize;
        let t = &mut t;
        let mut blocks = Vec::new();
        for i in 0..layers {
            let p = format!("blocks.{i}");
            blocks.push(Block {
                ln1: take_norm(t, &format!("{p}.ln1"))?,
                qkv: take_linear(t, &format!("{p}.qkv"))?,
                out: take_linear(t, &format!("{p}.out"))?,
                ln2: take_norm(t, &format!("{p}.ln2"))?,
                ff1: take_linear(t, &format!("{p}.ff1"))?,
                ff2: take_linear(t, &format!("{p}.ff2"))?,
            });
        }
        let (es, emb) = t.remove("emb.weight").ok_or("no emb.weight")?;
        let net = Tx {
            name: name.to_string(),
            emb,
            dim: es[1],
            heads,
            ent_in: take_linear(t, "ent_in")?,
            glob_in: take_linear(t, "glob_in")?,
            blocks,
            ln_f: take_norm(t, "ln_f")?,
            trunk1: take_linear(t, "trunk1")?,
            value: take_linear(t, "value")?,
            belief: take_linear(t, "belief")?,
            act_in: take_linear(t, "act_in")?,
            act_ln: take_norm(t, "act_ln")?,
            act_q: take_linear(t, "act_q")?,
            act_kv: take_linear(t, "act_kv")?,
            act_o: take_linear(t, "act_o")?,
            act_ff: take_linear(t, "act_ff")?,
            query: take_linear(t, "query")?,
            abias: take_linear(t, "abias")?,
        };
        if net.ent_in.inp != net.dim + ENT_F || net.glob_in.inp != GLOB_F || net.act_in.inp != net.dim + ACT_F {
            return Err(format!("{name}: made for other features than this build's"));
        }
        Ok(net)
    }

    fn embedding(&self, id: u32) -> &[f32] {
        let i = (id as usize).min(self.emb.len() / self.dim - 1);
        &self.emb[i * self.dim..(i + 1) * self.dim]
    }

    /// The tokens after the blocks, normed: the global one first
    fn tokens(&self, o: &Obs) -> Vec<Vec<f32>> {
        let mut x: Vec<Vec<f32>> = Vec::with_capacity(1 + o.n_ents());
        let mut y = Vec::new();
        self.glob_in.apply(&o.glob, &mut y, false);
        x.push(y.clone());
        let mut inp = Vec::new();
        for i in 0..o.n_ents() {
            inp.clear();
            inp.extend_from_slice(self.embedding(o.ent_ids[i]));
            inp.extend_from_slice(&o.ents[i * ENT_F..(i + 1) * ENT_F]);
            self.ent_in.apply(&inp, &mut y, false);
            x.push(y.clone());
        }
        let d = x[0].len();
        let (mut n, mut z, mut f) = (Vec::new(), Vec::new(), Vec::new());
        for b in &self.blocks {
            let (mut q, mut k, mut v) = (Vec::new(), Vec::new(), Vec::new());
            for row in &x {
                b.ln1.apply(row, &mut n);
                b.qkv.apply(&n, &mut z, false);
                q.push(z[..d].to_vec());
                k.push(z[d..2 * d].to_vec());
                v.push(z[2 * d..].to_vec());
            }
            let att = attend(&q, &k, &v, self.heads);
            for (row, a) in x.iter_mut().zip(&att) {
                b.out.apply(a, &mut z, false);
                row.iter_mut().zip(&z).for_each(|(r, u)| *r += u);
                b.ln2.apply(row, &mut n);
                b.ff1.apply(&n, &mut f, true);
                b.ff2.apply(&f, &mut z, false);
                row.iter_mut().zip(&z).for_each(|(r, u)| *r += u);
            }
        }
        x.iter()
            .map(|row| {
                self.ln_f.apply(row, &mut n);
                n.clone()
            })
            .collect()
    }

    /// The state `h` (the global token and the mean of the entity tokens through one layer) and the value
    fn head(&self, t: &[Vec<f32>]) -> (Vec<f32>, f32) {
        let d = t[0].len();
        let mut mean = vec![0f32; d];
        let ne = (t.len() - 1).max(1) as f32;
        for row in &t[1..] {
            mean.iter_mut().zip(row).for_each(|(m, r)| *m += r / ne);
        }
        let mut inp = t[0].clone();
        inp.extend(mean);
        let mut h = Vec::new();
        self.trunk1.apply(&inp, &mut h, true);
        let mut v = Vec::new();
        self.value.apply(&h, &mut v, false);
        (h, v[0])
    }

    pub fn state(&self, o: &Obs) -> (Vec<f32>, f32) {
        self.head(&self.tokens(o))
    }

    pub fn forward(&self, o: &Obs) -> Output {
        let t = self.tokens(o);
        let (h, value) = self.head(&t);
        let d = t[0].len();
        let (mut k, mut v, mut z) = (Vec::new(), Vec::new(), Vec::new());
        for row in &t {
            self.act_kv.apply(row, &mut z, false);
            k.push(z[..d].to_vec());
            v.push(z[d..].to_vec());
        }
        let mut q = Vec::new();
        self.query.apply(&h, &mut q, false);
        let (mut inp, mut a, mut n, mut aq, mut b) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut logits = Vec::with_capacity(o.n_acts());
        for i in 0..o.n_acts() {
            inp.clear();
            inp.extend_from_slice(self.embedding(o.act_ids[i]));
            inp.extend_from_slice(&o.acts[i * ACT_F..(i + 1) * ACT_F]);
            self.act_in.apply(&inp, &mut a, false);
            self.act_ln.apply(&a, &mut n);
            self.act_q.apply(&n, &mut aq, false);
            let att = attend(std::slice::from_ref(&aq), &k, &v, self.heads);
            self.act_o.apply(&att[0], &mut z, false);
            a.iter_mut().zip(&z).for_each(|(x, u)| *x += u);
            self.act_ff.apply(&a, &mut z, true);
            a.iter_mut().zip(&z).for_each(|(x, u)| *x += u);
            self.abias.apply(&a, &mut b, false);
            logits.push(q.iter().zip(&a).map(|(x, y)| x * y).sum::<f32>() + b[0]);
        }
        Output { logits, value, h }
    }

    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        let mut q = Vec::new();
        self.belief.apply(h, &mut q, false);
        cards.iter().map(|c| self.embedding(*c).iter().zip(&q).map(|(a, b)| a * b).sum()).collect()
    }
}

/// A network: the first, pooled one or the transformer
#[derive(Clone, Debug)]
pub enum Net {
    Mlp(Mlp),
    Tx(Tx),
}

impl Net {
    /// Reads a `.pmnet` file's bytes (the header's `arch` says which network)
    pub fn parse(name: &str, bytes: &[u8]) -> Result<Net, String> {
        let (header, _) = read(bytes)?;
        if header["arch"].as_str() == Some("tx") {
            Ok(Net::Tx(Tx::parse(name, bytes)?))
        } else {
            Ok(Net::Mlp(Mlp::parse(name, bytes)?))
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Net::Mlp(n) => &n.name,
            Net::Tx(n) => &n.name,
        }
    }

    /// The state `h` and the value
    pub fn state(&self, o: &Obs) -> (Vec<f32>, f32) {
        match self {
            Net::Mlp(n) => n.state(o),
            Net::Tx(n) => n.state(o),
        }
    }

    /// Every action's logit, and the value
    pub fn forward(&self, o: &Obs) -> Output {
        match self {
            Net::Mlp(n) => n.forward(o),
            Net::Tx(n) => n.forward(o),
        }
    }

    pub fn has_belief(&self) -> bool {
        match self {
            Net::Mlp(n) => n.belief.is_some(),
            Net::Tx(_) => true,
        }
    }

    /// Each card's logit of being in the opponent's hand
    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        match self {
            Net::Mlp(n) => n.belief(h, cards),
            Net::Tx(n) => n.belief(h, cards),
        }
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
