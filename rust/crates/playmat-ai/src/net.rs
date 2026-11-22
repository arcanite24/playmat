//! A small policy and value network, run on the CPU in plain Rust (#390 Track B). Trained by `rust/rl` (PyTorch),
//! written as a `.pmnet` file: one line of JSON naming each tensor and its shape, then the tensors as little-endian
//! f32 in that order.
//!
//! Fast CPU execution:
//! - Linear weights are repacked at load time into `[out_blocks, inp, 8]` panels (`wp`) for a 4x8 register-blocked
//!   SIMD GEMM (`matmul_rows`) that vectorizes cleanly to AVX2 FMA on x86_64 and NEON FMLA on ARM64 without horizontal
//!   reductions.
//! - `Tx::forward` and `Tx::state` reuse a thread-local flat `TxWorkspace` (zero per-layer heap allocations and no
//!   `Vec<Vec<f32>>` Q/K/V copies).
//! - Supports both v1 `Tx` checkpoints (ReLU + linear value head) and upgraded `Tx` checkpoints (`val_mlp=1` dedicated
//!   2-layer value head reading `[t0, mean_ent, max_ent, glob]`, and `gelu=1` tanh-approx GELU).

use crate::obs::{self, Obs, ACT_F, ENT_F, GLOB_F};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

#[inline(always)]
fn act_val(x: f32, gelu: bool) -> f32 {
    if gelu {
        // Matches PyTorch F.gelu(x, approximate="tanh")
        const K: f32 = 0.797_884_6; // sqrt(2 / pi)
        0.5 * x * (1.0 + (K * (x + 0.044_715 * x * x * x)).tanh())
    } else {
        x.max(0.0)
    }
}

/// A dense layer: `w` is `[out][in]` flat, plus `wp` repacked into `[out/8][in][8]` panels for 4x8 SIMD GEMM.
#[derive(Clone, Debug, Default)]
pub struct Linear {
    pub w: Vec<f32>,
    pub b: Vec<f32>,
    pub inp: usize,
    pub out: usize,
    /// Repacked weights: `blocks = out / 8` panels of `inp * 8` floats (contiguous 8 output lanes per `k`).
    wp: Vec<f32>,
}

impl Linear {
    pub fn new(w: Vec<f32>, b: Vec<f32>, out: usize, inp: usize) -> Linear {
        let blocks = out / 8;
        let mut wp = vec![0f32; blocks * inp * 8];
        for blk in 0..blocks {
            let ob = blk * 8;
            let dst_blk = &mut wp[blk * inp * 8..(blk + 1) * inp * 8];
            for k in 0..inp {
                let dst = &mut dst_blk[k * 8..(k + 1) * 8];
                for j in 0..8 {
                    dst[j] = w[(ob + j) * inp + k];
                }
            }
        }
        Linear { w, b, inp, out, wp }
    }

    /// Single-row apply into `y` (`y` resized to `self.out`).
    #[inline]
    fn apply(&self, x: &[f32], y: &mut Vec<f32>, relu: bool) {
        self.apply_act(x, y, relu, false);
    }

    fn apply_act(&self, x: &[f32], y: &mut Vec<f32>, do_act: bool, gelu: bool) {
        debug_assert_eq!(x.len(), self.inp);
        y.resize(self.out, 0.0);
        let inp = self.inp;
        let blocks = self.out / 8;
        for blk in 0..blocks {
            let ob = blk * 8;
            let mut acc = [
                self.b[ob],
                self.b[ob + 1],
                self.b[ob + 2],
                self.b[ob + 3],
                self.b[ob + 4],
                self.b[ob + 5],
                self.b[ob + 6],
                self.b[ob + 7],
            ];
            let panel = &self.wp[blk * inp * 8..(blk + 1) * inp * 8];
            for (k, &xk) in x.iter().enumerate() {
                let w = &panel[k * 8..(k + 1) * 8];
                for j in 0..8 {
                    acc[j] += xk * w[j];
                }
            }
            if do_act {
                for j in 0..8 {
                    y[ob + j] = act_val(acc[j], gelu);
                }
            } else {
                y[ob..ob + 8].copy_from_slice(&acc);
            }
        }
        for o in blocks * 8..self.out {
            let row = &self.w[o * inp..(o + 1) * inp];
            let s = self.b[o] + dot(row, x);
            y[o] = if do_act { act_val(s, gelu) } else { s };
        }
    }

    /// Flat batched matmul: `x` is `[rows, inp]`, writes `y` as `[rows, out]`.
    /// Uses `matrixmultiply::sgemm` (AVX2+FMA / NEON cache-blocked microkernels) for multi-row projections,
    /// and the packed 8-lane kernel for single rows.
    fn matmul_rows(&self, x: &[f32], rows: usize, y: &mut Vec<f32>, do_act: bool, gelu: bool, add_to_y: bool) {
        let inp = self.inp;
        let out = self.out;
        debug_assert_eq!(x.len(), rows * inp);
        if rows == 0 {
            if !add_to_y {
                y.clear();
            }
            return;
        }
        if rows == 1 && !add_to_y {
            self.apply_act(x, y, do_act, gelu);
            return;
        }
        if !add_to_y {
            y.resize(rows * out, 0.0);
        } else {
            debug_assert_eq!(y.len(), rows * out);
            debug_assert!(!do_act, "add_to_y with activation not supported");
        }
        let beta = if add_to_y { 1.0 } else { 0.0 };
        unsafe {
            matrixmultiply::sgemm(
                rows,
                inp,
                out,
                1.0,
                x.as_ptr(),
                inp as isize,
                1,
                self.w.as_ptr(),
                1,
                inp as isize,
                beta,
                y.as_mut_ptr(),
                out as isize,
                1,
            );
        }
        if do_act {
            if gelu {
                for r in 0..rows {
                    let row = &mut y[r * out..(r + 1) * out];
                    for (dst, &b) in row.iter_mut().zip(&self.b) {
                        *dst = act_val(*dst + b, true);
                    }
                }
            } else {
                for r in 0..rows {
                    let row = &mut y[r * out..(r + 1) * out];
                    for (dst, &b) in row.iter_mut().zip(&self.b) {
                        *dst = (*dst + b).max(0.0);
                    }
                }
            }
        } else {
            for r in 0..rows {
                let row = &mut y[r * out..(r + 1) * out];
                for (dst, &b) in row.iter_mut().zip(&self.b) {
                    *dst += b;
                }
            }
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
        let (_, mut tensors) = read(bytes)?;
        let mut take = |base: &str| -> Result<Linear, String> { take_linear(&mut tensors, base) };
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
        let (mut x, mut y, mut z) = (Vec::with_capacity(self.ent1.inp), Vec::new(), Vec::new());
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
        let (mut x, mut y, mut a, mut b) = (Vec::with_capacity(self.act1.inp), Vec::new(), Vec::new(), Vec::new());
        let mut logits = Vec::with_capacity(o.n_acts());
        for i in 0..o.n_acts() {
            x.clear();
            x.extend_from_slice(self.embedding(o.act_ids[i]));
            x.extend_from_slice(&o.acts[i * ACT_F..(i + 1) * ACT_F]);
            self.act1.apply(&x, &mut y, true);
            self.act2.apply(&y, &mut a, false);
            self.abias.apply(&a, &mut b, false);
            logits.push(dot(&q, &a) + b[0]);
        }
        Output { logits, value, h }
    }

    /// Each card's logit of being in the opponent's hand, from the state `h` (0 for a net without a belief head)
    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        let Some(bl) = &self.belief else { return vec![0.0; cards.len()] };
        let mut q = Vec::new();
        bl.apply(h, &mut q, false);
        cards.iter().map(|c| dot(self.embedding(*c), &q)).collect()
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
    #[inline]
    fn apply_slice(&self, x: &[f32], y: &mut [f32]) {
        let n = x.len() as f32;
        let mean = x.iter().sum::<f32>() / n;
        let var = x.iter().map(|&v| { let d = v - mean; d * d }).sum::<f32>() / n;
        let inv = 1.0 / (var + 1e-5).sqrt();
        for ((&v, (&w, &b)), dst) in x.iter().zip(self.w.iter().zip(&self.b)).zip(y.iter_mut()) {
            *dst = (v - mean) * inv * w + b;
        }
    }

    fn apply_rows(&self, x: &[f32], rows: usize, y: &mut Vec<f32>) {
        let d = self.w.len();
        y.resize(rows * d, 0.0);
        for r in 0..rows {
            self.apply_slice(&x[r * d..(r + 1) * d], &mut y[r * d..(r + 1) * d]);
        }
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

#[derive(Clone, Debug)]
pub struct ValMlp {
    pub ln: LayerNorm,
    pub fc1: Linear,
    pub fc2: Linear,
}

/// Reusable thread-local scratch buffers for `Tx::forward` / `Tx::state` (zero heap allocs in the hot loop).
#[derive(Default)]
struct TxWorkspace {
    ent_in_buf: Vec<f32>,
    x: Vec<f32>,
    norm: Vec<f32>,
    qkv: Vec<f32>,
    att: Vec<f32>,
    ff: Vec<f32>,
    scores: Vec<f32>,
    act_in_buf: Vec<f32>,
    act: Vec<f32>,
    act_norm: Vec<f32>,
    act_q: Vec<f32>,
    act_kv: Vec<f32>,
    act_att: Vec<f32>,
    act_bias: Vec<f32>,
    tmp1: Vec<f32>,
    tmp2: Vec<f32>,
    tmp3: Vec<f32>,
}

thread_local! {
    static TX_WS: RefCell<TxWorkspace> = RefCell::new(TxWorkspace::default());
}

/// The transformer network (#400 R2; `rust/rl/model.py` `Tx`): a global token and one per entity through pre-norm
/// self-attention blocks, then each candidate attending to the tokens once.
#[derive(Clone, Debug, Default)]
pub struct Tx {
    pub name: String,
    pub emb: Vec<f32>,
    pub dim: usize,
    pub d: usize,
    pub heads: usize,
    pub gelu: bool,
    pub ent_in: Linear,
    pub glob_in: Linear,
    pub blocks: Vec<Block>,
    pub ln_f: LayerNorm,
    pub trunk1: Linear,
    pub value: Option<Linear>,
    pub val_mlp: Option<ValMlp>,
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

/// Flat multi-head attention reading strided Q, K, V directly from buffers (`qkv` or `act_q`/`act_kv`).
#[allow(clippy::too_many_arguments)]
fn attend_strided(
    q: &[f32],
    q_stride: usize,
    q_off: usize,
    nq: usize,
    k: &[f32],
    k_stride: usize,
    k_off: usize,
    v: &[f32],
    v_stride: usize,
    v_off: usize,
    nk: usize,
    d: usize,
    heads: usize,
    out: &mut Vec<f32>,
    scores: &mut Vec<f32>,
) {
    let hd = d / heads;
    let scale = 1.0 / (hd as f32).sqrt();
    out.resize(nq * d, 0.0);
    out.fill(0.0);
    scores.resize(nk, 0.0);
    for qi in 0..nq {
        let q_base = qi * q_stride + q_off;
        let out_base = qi * d;
        for h in 0..heads {
            let ho = h * hd;
            let qr = &q[q_base + ho..q_base + ho + hd];
            let mut m = f32::MIN;
            for j in 0..nk {
                let kr = &k[j * k_stride + k_off + ho..j * k_stride + k_off + ho + hd];
                let s = dot(qr, kr) * scale;
                scores[j] = s;
                m = m.max(s);
            }
            let mut z = 0.0f32;
            for s in scores.iter_mut() {
                *s = (*s - m).exp();
                z += *s;
            }
            let inv_z = 1.0 / z;
            let o = &mut out[out_base + ho..out_base + ho + hd];
            for j in 0..nk {
                let p = scores[j] * inv_z;
                let vr = &v[j * v_stride + v_off + ho..j * v_stride + v_off + ho + hd];
                for (oo, &vv) in o.iter_mut().zip(vr) {
                    *oo += p * vv;
                }
            }
        }
    }
}

fn take_linear(t: &mut Tensors, b: &str) -> Result<Linear, String> {
    let (ws, w) = t.remove(&format!("{b}.weight")).ok_or(format!("no {b}.weight"))?;
    let (_, bb) = t.remove(&format!("{b}.bias")).ok_or(format!("no {b}.bias"))?;
    Ok(Linear::new(w, bb, ws[0], ws[1]))
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
        let d = cfg["d"].as_u64().ok_or("no d")? as usize;
        let heads = cfg["heads"].as_u64().ok_or("no heads")? as usize;
        let layers = cfg["layers"].as_u64().ok_or("no layers")? as usize;
        let gelu = cfg["gelu"].as_u64().unwrap_or(0) != 0;
        let use_val_mlp = cfg["val_mlp"].as_u64().unwrap_or(0) != 0;
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
        let (value, val_mlp) = if use_val_mlp {
            (
                None,
                Some(ValMlp {
                    ln: take_norm(t, "val_ln")?,
                    fc1: take_linear(t, "val_fc1")?,
                    fc2: take_linear(t, "val_fc2")?,
                }),
            )
        } else {
            (Some(take_linear(t, "value")?), None)
        };
        let net = Tx {
            name: name.to_string(),
            emb,
            dim: es[1],
            d,
            heads,
            gelu,
            ent_in: take_linear(t, "ent_in")?,
            glob_in: take_linear(t, "glob_in")?,
            blocks,
            ln_f: take_norm(t, "ln_f")?,
            trunk1: take_linear(t, "trunk1")?,
            value,
            val_mlp,
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

    #[inline]
    fn embedding(&self, id: u32) -> &[f32] {
        let i = (id as usize).min(self.emb.len() / self.dim - 1);
        &self.emb[i * self.dim..(i + 1) * self.dim]
    }

    /// Runs the transformer encoder into `ws.norm` (shape `[1 + n_ents, d]`), returning `nt = 1 + n_ents`.
    fn run_tokens(&self, o: &Obs, ws: &mut TxWorkspace) -> usize {
        let n_ents = o.n_ents();
        let nt = 1 + n_ents;
        let d = self.d;
        ws.x.resize(nt * d, 0.0);
        // Row 0: global token
        self.glob_in.apply_act(&o.glob, &mut ws.tmp1, false, false);
        ws.x[..d].copy_from_slice(&ws.tmp1);
        // Rows 1..nt: entity tokens
        if n_ents > 0 {
            let ein = self.dim + ENT_F;
            ws.ent_in_buf.resize(n_ents * ein, 0.0);
            for i in 0..n_ents {
                let dst = &mut ws.ent_in_buf[i * ein..(i + 1) * ein];
                dst[..self.dim].copy_from_slice(self.embedding(o.ent_ids[i]));
                dst[self.dim..].copy_from_slice(&o.ents[i * ENT_F..(i + 1) * ENT_F]);
            }
            self.ent_in.matmul_rows(&ws.ent_in_buf, n_ents, &mut ws.tmp2, false, false, false);
            ws.x[d..].copy_from_slice(&ws.tmp2);
        }
        for b in &self.blocks {
            b.ln1.apply_rows(&ws.x, nt, &mut ws.norm);
            b.qkv.matmul_rows(&ws.norm, nt, &mut ws.qkv, false, false, false);
            attend_strided(
                &ws.qkv,
                3 * d,
                0,
                nt,
                &ws.qkv,
                3 * d,
                d,
                &ws.qkv,
                3 * d,
                2 * d,
                nt,
                d,
                self.heads,
                &mut ws.att,
                &mut ws.scores,
            );
            b.out.matmul_rows(&ws.att, nt, &mut ws.x, false, false, true);
            b.ln2.apply_rows(&ws.x, nt, &mut ws.norm);
            b.ff1.matmul_rows(&ws.norm, nt, &mut ws.ff, true, self.gelu, false);
            b.ff2.matmul_rows(&ws.ff, nt, &mut ws.x, false, false, true);
        }
        self.ln_f.apply_rows(&ws.x, nt, &mut ws.norm);
        nt
    }

    /// Computes state `h` and value logit from final normed tokens in `ws.norm` (`[nt, d]`).
    fn run_head(&self, o: &Obs, nt: usize, ws: &mut TxWorkspace) -> (Vec<f32>, f32) {
        let d = self.d;
        let t = &ws.norm;
        let n_ents = nt - 1;
        ws.tmp1.resize(2 * d, 0.0);
        ws.tmp1[..d].copy_from_slice(&t[..d]);
        let mean = &mut ws.tmp1[d..2 * d];
        mean.fill(0.0);
        if n_ents > 0 {
            let inv = 1.0 / n_ents as f32;
            for i in 1..nt {
                let row = &t[i * d..(i + 1) * d];
                for (m, &r) in mean.iter_mut().zip(row) {
                    *m += r * inv;
                }
            }
        }
        let mut h = Vec::with_capacity(self.trunk1.out);
        self.trunk1.apply_act(&ws.tmp1, &mut h, true, self.gelu);
        let value = if let Some(vm) = &self.val_mlp {
            let vin = 3 * d + GLOB_F;
            ws.tmp2.resize(vin, 0.0);
            ws.tmp2[..2 * d].copy_from_slice(&ws.tmp1[..2 * d]);
            let mx = &mut ws.tmp2[2 * d..3 * d];
            if n_ents > 0 {
                mx.copy_from_slice(&t[d..2 * d]);
                for i in 2..nt {
                    let row = &t[i * d..(i + 1) * d];
                    for (m, &r) in mx.iter_mut().zip(row) {
                        if r > *m {
                            *m = r;
                        }
                    }
                }
            } else {
                mx.fill(0.0);
            }
            ws.tmp2[3 * d..].copy_from_slice(&o.glob);
            ws.tmp3.resize(vin, 0.0);
            vm.ln.apply_slice(&ws.tmp2, &mut ws.tmp3);
            vm.fc1.apply_act(&ws.tmp3, &mut ws.tmp1, true, self.gelu);
            vm.fc2.apply_act(&ws.tmp1, &mut ws.tmp2, false, false);
            ws.tmp2[0]
        } else {
            self.value.as_ref().unwrap().apply_act(&h, &mut ws.tmp2, false, false);
            ws.tmp2[0]
        };
        (h, value)
    }

    pub fn state(&self, o: &Obs) -> (Vec<f32>, f32) {
        TX_WS.with(|cell| {
            let mut guard = cell.borrow_mut();
            let ws = &mut *guard;
            let nt = self.run_tokens(o, ws);
            self.run_head(o, nt, ws)
        })
    }

    pub fn forward(&self, o: &Obs) -> Output {
        TX_WS.with(|cell| {
            let mut guard = cell.borrow_mut();
            let ws = &mut *guard;
            let nt = self.run_tokens(o, ws);
            let (h, value) = self.run_head(o, nt, ws);
            let na = o.n_acts();
            if na == 0 {
                return Output { logits: Vec::new(), value, h };
            }
            let d = self.d;
            // Key/Value over all tokens
            self.act_kv.matmul_rows(&ws.norm, nt, &mut ws.act_kv, false, false, false);
            // Query from state h
            self.query.apply_act(&h, &mut ws.tmp1, false, false);
            // Action inputs
            let ain = self.dim + ACT_F;
            ws.act_in_buf.resize(na * ain, 0.0);
            for i in 0..na {
                let dst = &mut ws.act_in_buf[i * ain..(i + 1) * ain];
                dst[..self.dim].copy_from_slice(self.embedding(o.act_ids[i]));
                dst[self.dim..].copy_from_slice(&o.acts[i * ACT_F..(i + 1) * ACT_F]);
            }
            self.act_in.matmul_rows(&ws.act_in_buf, na, &mut ws.act, false, false, false);
            self.act_ln.apply_rows(&ws.act, na, &mut ws.act_norm);
            self.act_q.matmul_rows(&ws.act_norm, na, &mut ws.act_q, false, false, false);
            attend_strided(
                &ws.act_q,
                d,
                0,
                na,
                &ws.act_kv,
                2 * d,
                0,
                &ws.act_kv,
                2 * d,
                d,
                nt,
                d,
                self.heads,
                &mut ws.act_att,
                &mut ws.scores,
            );
            self.act_o.matmul_rows(&ws.act_att, na, &mut ws.act, false, false, true);
            // Residual FFN: a = a + act(act_ff(a))
            self.act_ff.matmul_rows(&ws.act, na, &mut ws.act_norm, true, self.gelu, false);
            for (dst, &u) in ws.act.iter_mut().zip(&ws.act_norm) {
                *dst += u;
            }
            self.abias.matmul_rows(&ws.act, na, &mut ws.act_bias, false, false, false);
            let mut logits = Vec::with_capacity(na);
            for i in 0..na {
                logits.push(dot(&ws.tmp1, &ws.act[i * d..(i + 1) * d]) + ws.act_bias[i]);
            }
            Output { logits, value, h }
        })
    }

    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        let mut q = Vec::new();
        self.belief.apply_act(h, &mut q, false, false);
        cards.iter().map(|c| dot(self.embedding(*c), &q)).collect()
    }
}

struct PendingReq {
    obs: Obs,
    reply: Arc<(Mutex<Option<Output>>, Condvar)>,
}

struct BatchQueue {
    reqs: Vec<PendingReq>,
    closed: bool,
}

/// Batched TCP client for `rust/rl/infer_server.py` (GPU-accelerated self-play).
pub struct RemoteNet {
    pub name: String,
    queue: Arc<(Mutex<BatchQueue>, Condvar)>,
    emb: Vec<f32>,
    dim: usize,
    belief: Option<Linear>,
}

impl std::fmt::Debug for RemoteNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteNet").field("name", &self.name).field("dim", &self.dim).finish()
    }
}

fn read_u32(s: &mut TcpStream) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    s.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_f32s(s: &mut TcpStream, n: usize) -> std::io::Result<Vec<f32>> {
    let mut bytes = vec![0u8; n * 4];
    s.read_exact(&mut bytes)?;
    Ok(bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

impl RemoteNet {
    pub fn connect(url: &str) -> Result<RemoteNet, String> {
        let addr = url.strip_prefix("tcp://").ok_or("expected tcp://host:port")?;
        let mut stream = TcpStream::connect(addr).map_err(|e| format!("{url}: {e}"))?;
        stream.set_nodelay(true).ok();
        let dim = read_u32(&mut stream).map_err(|e| format!("{url} handshake: {e}"))? as usize;
        let buckets = read_u32(&mut stream).map_err(|e| format!("{url} handshake: {e}"))? as usize;
        let trunk = read_u32(&mut stream).map_err(|e| format!("{url} handshake: {e}"))? as usize;
        let mut has_b = [0u8; 1];
        stream.read_exact(&mut has_b).map_err(|e| format!("{url} handshake: {e}"))?;
        let (emb, belief) = if has_b[0] != 0 {
            let emb = read_f32s(&mut stream, buckets * dim).map_err(|e| format!("{url} emb: {e}"))?;
            let bw = read_f32s(&mut stream, dim * trunk).map_err(|e| format!("{url} bw: {e}"))?;
            let bb = read_f32s(&mut stream, dim).map_err(|e| format!("{url} bb: {e}"))?;
            (emb, Some(Linear::new(bw, bb, dim, trunk)))
        } else {
            (Vec::new(), None)
        };

        let queue = Arc::new((Mutex::new(BatchQueue { reqs: Vec::new(), closed: false }), Condvar::new()));
        let q_bg = Arc::clone(&queue);
        let max_batch: usize = std::env::var("PLAYMAT_GPU_BATCH").ok().and_then(|v| v.parse().ok()).unwrap_or(64);
        // Default 0: never call wait_timeout on Windows (where sub-15.6ms sleeps round up to 15.6ms!);
        // natural coalescing happens while the previous GPU batch is in flight.
        let wait_us: u64 = std::env::var("PLAYMAT_GPU_WAIT_US").ok().and_then(|v| v.parse().ok()).unwrap_or(0);

        std::thread::spawn(move || {
            let mut payload = Vec::<u8>::with_capacity(2 << 20);
            let mut resp_buf = Vec::<u8>::with_capacity(1 << 18);
            loop {
                let batch = {
                    let (lock, cv) = &*q_bg;
                    let mut g = lock.lock().unwrap();
                    while g.reqs.is_empty() && !g.closed {
                        g = cv.wait(g).unwrap();
                    }
                    if g.reqs.is_empty() && g.closed {
                        break;
                    }
                    if g.reqs.len() < max_batch && wait_us > 0 {
                        let deadline = Instant::now() + Duration::from_micros(wait_us);
                        while g.reqs.len() < max_batch && !g.closed {
                            let rem = deadline.saturating_duration_since(Instant::now());
                            if rem.is_zero() {
                                break;
                            }
                            let (ng, to) = cv.wait_timeout(g, rem).unwrap();
                            g = ng;
                            if to.timed_out() {
                                break;
                            }
                        }
                    }
                    let take = g.reqs.len().min(max_batch);
                    g.reqs.drain(..take).collect::<Vec<_>>()
                };

                let b = batch.len();
                let max_e = batch.iter().map(|r| r.obs.n_ents()).max().unwrap_or(0);
                let max_a = batch.iter().map(|r| r.obs.n_acts()).max().unwrap_or(0);
                let w_e = ((max_e + 7) / 8 * 8).max(8);
                let w_a = ((max_a + 3) / 4 * 4).max(4);

                // Pre-pack 7 flat contiguous arrays so Python infer_server.py runs with ZERO Python loops
                let glob_bytes = b * GLOB_F * 4;
                let eids_bytes = b * w_e * 2;
                let ents_bytes = b * w_e * ENT_F * 4;
                let em_bytes = b * w_e;
                let aids_bytes = b * w_a * 2;
                let acts_bytes = b * w_a * ACT_F * 4;
                let am_bytes = b * w_a;
                let total_bytes = 12 + glob_bytes + eids_bytes + ents_bytes + em_bytes + aids_bytes + acts_bytes + am_bytes;
                payload.clear();
                payload.resize(total_bytes, 0);
                payload[0..4].copy_from_slice(&(b as u32).to_le_bytes());
                payload[4..8].copy_from_slice(&(w_e as u32).to_le_bytes());
                payload[8..12].copy_from_slice(&(w_a as u32).to_le_bytes());

                let o_glob = 12;
                let o_eids = o_glob + glob_bytes;
                let o_ents = o_eids + eids_bytes;
                let o_em = o_ents + ents_bytes;
                let o_aids = o_em + em_bytes;
                let o_acts = o_aids + aids_bytes;
                let o_am = o_acts + acts_bytes;

                for (i, req) in batch.iter().enumerate() {
                    let o = &req.obs;
                    // glob
                    let gd = &mut payload[o_glob + i * GLOB_F * 4..o_glob + (i + 1) * GLOB_F * 4];
                    for (k, &v) in o.glob.iter().enumerate() {
                        gd[k * 4..(k + 1) * 4].copy_from_slice(&v.to_le_bytes());
                    }
                    // entities
                    let ne = o.n_ents().min(w_e);
                    if ne > 0 {
                        let id_base = o_eids + i * w_e * 2;
                        for k in 0..ne {
                            payload[id_base + k * 2..id_base + k * 2 + 2]
                                .copy_from_slice(&(o.ent_ids[k] as u16).to_le_bytes());
                        }
                        let ef_base = o_ents + i * w_e * ENT_F * 4;
                        for (k, &v) in o.ents[..ne * ENT_F].iter().enumerate() {
                            payload[ef_base + k * 4..ef_base + k * 4 + 4].copy_from_slice(&v.to_le_bytes());
                        }
                        let em_base = o_em + i * w_e;
                        payload[em_base..em_base + ne].fill(1);
                    }
                    // actions
                    let na = o.n_acts().min(w_a);
                    if na > 0 {
                        let id_base = o_aids + i * w_a * 2;
                        for k in 0..na {
                            payload[id_base + k * 2..id_base + k * 2 + 2]
                                .copy_from_slice(&(o.act_ids[k] as u16).to_le_bytes());
                        }
                        let af_base = o_acts + i * w_a * ACT_F * 4;
                        for (k, &v) in o.acts[..na * ACT_F].iter().enumerate() {
                            payload[af_base + k * 4..af_base + k * 4 + 4].copy_from_slice(&v.to_le_bytes());
                        }
                        let am_base = o_am + i * w_a;
                        payload[am_base..am_base + na].fill(1);
                    }
                }
                if stream.write_all(&payload).is_err() {
                    break;
                }
                let Ok(rb) = read_u32(&mut stream) else { break };
                let Ok(rtrunk) = read_u32(&mut stream) else { break };
                debug_assert_eq!(rb as usize, b);
                let trunk = rtrunk as usize;
                let need = (b + b * w_a + b * trunk) * 4;
                resp_buf.resize(need, 0);
                if stream.read_exact(&mut resp_buf).is_err() {
                    break;
                }
                let floats: Vec<f32> = resp_buf.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect();
                let vs = &floats[..b];
                let all_logits = &floats[b..b + b * w_a];
                let all_h = &floats[b + b * w_a..];
                for (i, req) in batch.into_iter().enumerate() {
                    let na = req.obs.n_acts().min(w_a);
                    let logits = all_logits[i * w_a..i * w_a + na].to_vec();
                    let h = all_h[i * trunk..(i + 1) * trunk].to_vec();
                    let (rlock, rcv) = &*req.reply;
                    *rlock.lock().unwrap() = Some(Output { logits, value: vs[i], h });
                    rcv.notify_one();
                }
            }
        });

        Ok(RemoteNet { name: url.to_string(), queue, emb, dim, belief })
    }

    pub fn forward(&self, o: &Obs) -> Output {
        let reply = Arc::new((Mutex::new(None), Condvar::new()));
        {
            let (lock, cv) = &*self.queue;
            let mut g = lock.lock().unwrap();
            g.reqs.push(PendingReq { obs: o.clone(), reply: Arc::clone(&reply) });
            cv.notify_one();
        }
        let (rlock, rcv) = &*reply;
        let mut rg = rlock.lock().unwrap();
        while rg.is_none() {
            rg = rcv.wait(rg).unwrap();
        }
        rg.take().unwrap()
    }

    pub fn state(&self, o: &Obs) -> (Vec<f32>, f32) {
        let out = self.forward(o);
        (out.h, out.value)
    }

    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        let Some(bl) = &self.belief else { return vec![0.0; cards.len()] };
        let mut q = Vec::new();
        bl.apply(h, &mut q, false);
        let n_emb = self.emb.len() / self.dim.max(1);
        cards
            .iter()
            .map(|c| {
                let i = (*c as usize).min(n_emb.saturating_sub(1));
                dot(&self.emb[i * self.dim..(i + 1) * self.dim], &q)
            })
            .collect()
    }
}

/// A network: the first, pooled one, the transformer, or a GPU-batched remote server (`tcp://...`)
#[derive(Clone, Debug)]
pub enum Net {
    Mlp(Mlp),
    Tx(Tx),
    Remote(Arc<RemoteNet>),
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
            Net::Remote(n) => &n.name,
        }
    }

    /// The state `h` and the value
    pub fn state(&self, o: &Obs) -> (Vec<f32>, f32) {
        match self {
            Net::Mlp(n) => n.state(o),
            Net::Tx(n) => n.state(o),
            Net::Remote(n) => n.state(o),
        }
    }

    /// Every action's logit, and the value
    pub fn forward(&self, o: &Obs) -> Output {
        match self {
            Net::Mlp(n) => n.forward(o),
            Net::Tx(n) => n.forward(o),
            Net::Remote(n) => n.forward(o),
        }
    }

    pub fn has_belief(&self) -> bool {
        match self {
            Net::Mlp(n) => n.belief.is_some(),
            Net::Tx(_) => true,
            Net::Remote(n) => n.belief.is_some(),
        }
    }

    /// Each card's logit of being in the opponent's hand
    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        match self {
            Net::Mlp(n) => n.belief(h, cards),
            Net::Tx(n) => n.belief(h, cards),
            Net::Remote(n) => n.belief(h, cards),
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

/// The network at `path`, read once per process (`builtin`: shipped net; `tcp://host:port`: GPU inference server)
pub fn load(path: &str) -> Result<Arc<Net>, String> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    // Hold the cache lock while loading/connecting so `--jobs 64` worker threads share ONE RemoteNet
    // TCP connection and coalesce into true B=32..64 GPU batches instead of opening 64 separate B=1 sockets!
    let mut map = cache.lock().unwrap();
    if let Some(n) = map.get(path) {
        return Ok(Arc::clone(n));
    }
    let net = if path == "builtin" {
        Net::parse("builtin", BUILTIN)?
    } else if path.starts_with("tcp://") {
        Net::Remote(Arc::new(RemoteNet::connect(path)?))
    } else {
        let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        Net::parse(path, &bytes)?
    };
    let net = Arc::new(net);
    map.insert(path.to_string(), Arc::clone(&net));
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
