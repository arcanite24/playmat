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

use crate::fm::Fm;
use crate::obs::{self, Obs, ACT_F, ENT_F, GLOB_F, GLOB_F_V1, GLOB_F_V2, GLOB_F_V3, NO_ENT, NUM_REL_TYPES};
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
        0.5 * x * (1.0 + (K * (x + 0.044_715 * x * x * x)).stanh())
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

    /// The strict build's matrix product: per output, the bias and then each input in order, exactly as `apply_act`
    /// sums a single row (no fused multiply-add: Rust never fuses on its own), eight rows at a time (then four, then
    /// one) so each weight panel is read once per block. `y` is sized already.
    #[cfg(feature = "strict")]
    fn strict_rows(&self, x: &[f32], rows: usize, y: &mut [f32], do_act: bool, gelu: bool, add_to_y: bool) {
        let inp = self.inp;
        let mut r0 = 0;
        while r0 < rows {
            let left = rows - r0;
            let n = if left >= 8 {
                self.strict_block::<8>(&x[r0 * inp..(r0 + 8) * inp], &mut y[r0 * self.out..], do_act, gelu, add_to_y);
                8
            } else if left >= 4 {
                self.strict_block::<4>(&x[r0 * inp..(r0 + 4) * inp], &mut y[r0 * self.out..], do_act, gelu, add_to_y);
                4
            } else {
                self.strict_block::<1>(&x[r0 * inp..(r0 + 1) * inp], &mut y[r0 * self.out..], do_act, gelu, add_to_y);
                1
            };
            r0 += n;
        }
    }

    /// `R` rows of the strict product: `x` is `[R, inp]`, `y` starts at the first of them
    #[cfg(feature = "strict")]
    #[inline(always)]
    fn strict_block<const R: usize>(&self, x: &[f32], y: &mut [f32], do_act: bool, gelu: bool, add_to_y: bool) {
        let (inp, out) = (self.inp, self.out);
        let blocks = out / 8;
        assert!(x.len() == R * inp && y.len() >= R * out);
        let xs: [&[f32]; R] = std::array::from_fn(|r| &x[r * inp..(r + 1) * inp]);
        let fin = |acc: f32| if do_act { act_val(acc, gelu) } else { acc };
        for blk in 0..blocks {
            let ob = blk * 8;
            let mut b8 = [0f32; 8];
            b8.copy_from_slice(&self.b[ob..ob + 8]);
            let mut acc = [b8; R];
            let panel = &self.wp[blk * inp * 8..(blk + 1) * inp * 8];
            for (k, w) in panel.as_chunks::<8>().0.iter().enumerate() {
                for r in 0..R {
                    let v = xs[r][k];
                    for j in 0..8 {
                        acc[r][j] += v * w[j];
                    }
                }
            }
            for (r, a) in acc.iter().enumerate() {
                let dst = &mut y[r * out + ob..r * out + ob + 8];
                for (d, &v) in dst.iter_mut().zip(a) {
                    let v = fin(v);
                    *d = if add_to_y { *d + v } else { v };
                }
            }
        }
        for (r, xr) in xs.iter().enumerate() {
            for o in blocks * 8..out {
                let v = fin(self.b[o] + dot(&self.w[o * inp..(o + 1) * inp], xr));
                let d = &mut y[r * out + o];
                *d = if add_to_y { *d + v } else { v };
            }
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
        // the strict build skips `matrixmultiply`, whose kernels (AVX2 and FMA, NEON) round differently per machine,
        // for the packed 8-lane loop: the same sums in the same order everywhere (crate::fm)
        #[cfg(feature = "strict")]
        {
            self.strict_rows(x, rows, y, do_act, gelu, add_to_y);
            return;
        }
        #[cfg(all(feature = "gpu-wgpu", not(feature = "strict")))]
        {
            static USE_WGPU: OnceLock<bool> = OnceLock::new();
            let use_wgpu = *USE_WGPU.get_or_init(|| std::env::var("PLAYMAT_WGPU").as_deref() == Ok("1"));
            if use_wgpu && rows * out * inp >= 65_536 {
                if let Ok(ctx) = crate::gpu_wgpu::WgpuContext::global() {
                    if ctx.matmul_rows(x, &self.w, &self.b, rows, inp, out, y, do_act, gelu, add_to_y).is_ok() {
                        return;
                    }
                }
            }
        }
        #[allow(unreachable_code)]
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
    /// optional per-action Q probability in (0, 1) from `TxV2` `act_q_head` (empty when model has no `act_q`)
    pub act_q: Vec<f32>,
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
        if ent1.inp != dim + ENT_F
            || (glob1.inp != GLOB_F && glob1.inp != GLOB_F_V3 && glob1.inp != GLOB_F_V2 && glob1.inp != GLOB_F_V1)
            || act1.inp != dim + ACT_F
        {
            return Err(format!(
                "{name}: made for other features (entity {}, global {}, action {}; this build has {}, {} or {}, {})",
                ent1.inp - dim,
                glob1.inp,
                act1.inp - dim,
                ENT_F,
                GLOB_F_V1,
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
        let mut gbuf = [0f32; GLOB_F];
        let gn = self.glob1.inp.min(o.glob.len());
        gbuf[..gn].copy_from_slice(&o.glob[..gn]);
        self.glob1.apply(&gbuf[..self.glob1.inp], &mut y, true);
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
        Output { logits, value, h, act_q: Vec::new() }
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
        let var = x
            .iter()
            .map(|&v| {
                let d = v - mean;
                d * d
            })
            .sum::<f32>()
            / n;
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
    rel_offs: Vec<u32>,
    rel_edges: Vec<(u16, u8)>,
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
    pub glob_f: usize,
    pub rel_bias: Option<Vec<f32>>,
    pub act_ent: bool,
    pub ent_in: Linear,
    pub glob_in: Linear,
    pub blocks: Vec<Block>,
    pub ln_f: LayerNorm,
    pub trunk1: Linear,
    pub value: Option<Linear>,
    pub val_mlp: Option<ValMlp>,
    pub belief: Linear,
    pub threat: Option<Linear>,
    pub act_in: Linear,
    pub act_ln: LayerNorm,
    pub act_q: Linear,
    pub act_kv: Linear,
    pub act_o: Linear,
    pub act_ff: Linear,
    pub query: Linear,
    pub abias: Linear,
}

type RelCsr<'a> = (&'a [u32], &'a [(u16, u8)], &'a [f32]);

/// Flat multi-head attention reading strided Q, K, V directly from buffers (`qkv` or `act_q`/`act_kv`).
/// Relational biases `rel` use pre-extracted CSR `(rel_offs, rel_edges, rb)` built once per observation.
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
    rel: Option<RelCsr<'_>>,
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
        let q_edges: &[(u16, u8)] = match rel {
            Some((offs, edges, _)) if qi + 1 < offs.len() => &edges[offs[qi] as usize..offs[qi + 1] as usize],
            _ => &[],
        };
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
            if let Some((_, _, rb)) = rel {
                for &(j_u16, row_u8) in q_edges {
                    let j = j_u16 as usize;
                    scores[j] += rb[row_u8 as usize * heads + h];
                    m = m.max(scores[j]);
                }
            }
            let mut z = 0.0f32;
            for s in scores.iter_mut() {
                *s = (*s - m).sexp();
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
        let use_rel = cfg["rel"].as_u64().unwrap_or(0) != 0;
        let act_ent = cfg["act_ent"].as_u64().unwrap_or(0) != 0;
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
        let rel_bias = if use_rel {
            let (rs, rb) = t.remove("rel_bias").ok_or("no rel_bias")?;
            if rs != vec![2 * NUM_REL_TYPES, heads] {
                return Err(format!("{name}: bad rel_bias shape {rs:?}"));
            }
            Some(rb)
        } else {
            None
        };
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
        let glob_in = take_linear(t, "glob_in")?;
        let glob_f = glob_in.inp;
        let net = Tx {
            name: name.to_string(),
            emb,
            dim: es[1],
            d,
            heads,
            gelu,
            glob_f,
            rel_bias,
            act_ent,
            ent_in: take_linear(t, "ent_in")?,
            glob_in,
            blocks,
            ln_f: take_norm(t, "ln_f")?,
            trunk1: take_linear(t, "trunk1")?,
            value,
            val_mlp,
            belief: take_linear(t, "belief")?,
            threat: take_linear(t, "threat").ok(),
            act_in: take_linear(t, "act_in")?,
            act_ln: take_norm(t, "act_ln")?,
            act_q: take_linear(t, "act_q")?,
            act_kv: take_linear(t, "act_kv")?,
            act_o: take_linear(t, "act_o")?,
            act_ff: take_linear(t, "act_ff")?,
            query: take_linear(t, "query")?,
            abias: take_linear(t, "abias")?,
        };
        let want_act = net.dim + ACT_F + if net.act_ent { net.d } else { 0 };
        if net.ent_in.inp != net.dim + ENT_F
            || (net.glob_f != GLOB_F && net.glob_f != GLOB_F_V3 && net.glob_f != GLOB_F_V2 && net.glob_f != GLOB_F_V1)
            || net.act_in.inp != want_act
        {
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
        // Row 0: global token (slice/pad to self.glob_f so v1=65 and v2=81 models both work)
        let mut gbuf = [0f32; GLOB_F];
        let gn = self.glob_f.min(o.glob.len());
        gbuf[..gn].copy_from_slice(&o.glob[..gn]);
        self.glob_in.apply_act(&gbuf[..self.glob_f], &mut ws.tmp1, false, false);
        ws.x[..d].copy_from_slice(&ws.tmp1);
        // Rows 1..nt: entity tokens (when an Obs v2 81-glob obs is fed to a legacy v1 glob_f=65 model, skip
        // graveyard/exile tokens that v1 never saw during training, while keeping legacy 65-glob parity fixtures exact)
        let v1_filter = self.glob_f == GLOB_F_V1 && o.glob.len() > GLOB_F_V1;
        let mut kept = 0usize;
        if n_ents > 0 {
            let ein = self.dim + ENT_F;
            ws.ent_in_buf.clear();
            ws.ent_in_buf.reserve(n_ents * ein);
            for i in 0..n_ents {
                let ef = &o.ents[i * ENT_F..(i + 1) * ENT_F];
                if v1_filter && (ef[4] > 0.5 || ef[5] > 0.5) {
                    continue;
                }
                ws.ent_in_buf.extend_from_slice(self.embedding(o.ent_ids[i]));
                ws.ent_in_buf.extend_from_slice(ef);
                kept += 1;
            }
            if kept > 0 {
                self.ent_in.matmul_rows(&ws.ent_in_buf, kept, &mut ws.tmp2, false, false, false);
                ws.x.resize((1 + kept) * d, 0.0);
                ws.x[d..(1 + kept) * d].copy_from_slice(&ws.tmp2);
            } else {
                ws.x.resize(d, 0.0);
            }
        }
        let nt = 1 + kept;
        ws.rel_offs.clear();
        ws.rel_edges.clear();
        let has_rel = if self.rel_bias.is_some() && o.ent_rel_idx.len() == n_ents && o.ent_rel_type.len() == n_ents {
            ws.rel_offs.resize(nt + 1, 0);
            let ne = n_ents.min(nt.saturating_sub(1));
            for qi in 1..nt {
                ws.rel_offs[qi] = ws.rel_edges.len() as u32;
                let k_ent = qi - 1;
                if k_ent < ne {
                    let rt = o.ent_rel_type[k_ent] as usize;
                    if (1..=NUM_REL_TYPES).contains(&rt) {
                        let tgt = o.ent_rel_idx[k_ent] as usize;
                        if tgt < ne {
                            ws.rel_edges.push(((1 + tgt) as u16, (rt - 1) as u8));
                        }
                    }
                    for j_ent in 0..ne {
                        let rt = o.ent_rel_type[j_ent] as usize;
                        if (1..=NUM_REL_TYPES).contains(&rt) && o.ent_rel_idx[j_ent] as usize == k_ent {
                            ws.rel_edges.push(((1 + j_ent) as u16, (NUM_REL_TYPES + rt - 1) as u8));
                        }
                    }
                }
            }
            ws.rel_offs[nt] = ws.rel_edges.len() as u32;
            true
        } else {
            false
        };
        for b in &self.blocks {
            b.ln1.apply_rows(&ws.x, nt, &mut ws.norm);
            b.qkv.matmul_rows(&ws.norm, nt, &mut ws.qkv, false, false, false);
            let rel = if has_rel {
                self.rel_bias.as_deref().map(|rb| (ws.rel_offs.as_slice(), ws.rel_edges.as_slice(), rb))
            } else {
                None
            };
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
                rel,
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
            let vin = 3 * d + self.glob_f;
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
            let gn = self.glob_f.min(o.glob.len());
            ws.tmp2[3 * d..3 * d + gn].copy_from_slice(&o.glob[..gn]);
            if gn < self.glob_f {
                ws.tmp2[3 * d + gn..vin].fill(0.0);
            }
            ws.tmp3.resize(vin, 0.0);
            vm.ln.apply_slice(&ws.tmp2, &mut ws.tmp3);
            vm.fc1.apply_act(&ws.tmp3, &mut ws.tmp1, true, self.gelu);
            vm.fc2.apply_act(&ws.tmp1, &mut ws.tmp2, false, false);
            if vm.fc2.out > 1 {
                let probs = softmax(&ws.tmp2[..vm.fc2.out], 1.0);
                let denom = (vm.fc2.out - 1) as f32;
                let mut mu = 0.0f32;
                for (i, &p) in probs.iter().enumerate() {
                    mu += p * (i as f32 / denom);
                }
                let mu_c = mu.clamp(1e-5, 1.0 - 1e-5);
                (mu_c / (1.0 - mu_c)).sln()
            } else {
                ws.tmp2[0]
            }
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
                return Output { logits: Vec::new(), value, h, act_q: Vec::new() };
            }
            let d = self.d;
            // Key/Value over all tokens
            self.act_kv.matmul_rows(&ws.norm, nt, &mut ws.act_kv, false, false, false);
            // Query from state h
            self.query.apply_act(&h, &mut ws.tmp1, false, false);
            // Action inputs (optionally concatenated with contextualized entity token `ws.norm[1 + act_ent_idx[i]]`)
            let n_ents = nt - 1;
            let ain = self.dim + ACT_F + if self.act_ent { d } else { 0 };
            ws.act_in_buf.resize(na * ain, 0.0);
            for i in 0..na {
                let dst = &mut ws.act_in_buf[i * ain..(i + 1) * ain];
                dst[..self.dim].copy_from_slice(self.embedding(o.act_ids[i]));
                dst[self.dim..self.dim + ACT_F].copy_from_slice(&o.acts[i * ACT_F..(i + 1) * ACT_F]);
                if self.act_ent {
                    let ei = o.act_ent_idx.get(i).copied().unwrap_or(NO_ENT) as usize;
                    let ctx = &mut dst[self.dim + ACT_F..];
                    if ei < n_ents {
                        ctx.copy_from_slice(&ws.norm[(1 + ei) * d..(2 + ei) * d]);
                    } else {
                        ctx.fill(0.0);
                    }
                }
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
                None,
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
            Output { logits, value, h, act_q: Vec::new() }
        })
    }

    pub fn belief(&self, h: &[f32], cards: &[u32]) -> Vec<f32> {
        let mut q = Vec::new();
        self.belief.apply_act(h, &mut q, false, false);
        cards.iter().map(|c| dot(self.embedding(*c), &q)).collect()
    }

    pub fn threat(&self, h: &[f32]) -> [f32; crate::obs::BELIEF_META_F] {
        let mut out = [0.0f32; crate::obs::BELIEF_META_F];
        if let Some(th) = &self.threat {
            let mut q = Vec::new();
            th.apply_act(h, &mut q, false, false);
            let n = out.len().min(q.len());
            out[..n].copy_from_slice(&q[..n]);
        }
        out
    }
}

struct BatchReplyState {
    rem: usize,
    outs: Vec<Output>,
}

type BatchSlot = Arc<(Mutex<BatchReplyState>, Condvar)>;

struct PendingReq {
    obs: Obs,
    need_h: bool,
    slot: BatchSlot,
    idx: usize,
}

struct InFlightItem {
    n_acts: usize,
    need_h: bool,
    slot: BatchSlot,
    idx: usize,
}

struct InFlightBatch {
    w_a: usize,
    items: Vec<InFlightItem>,
}

struct BatchQueue {
    reqs: Vec<PendingReq>,
    closed: bool,
}

enum RemoteStream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
}

impl RemoteStream {
    fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            RemoteStream::Tcp(s) => Ok(RemoteStream::Tcp(s.try_clone()?)),
            #[cfg(unix)]
            RemoteStream::Unix(s) => Ok(RemoteStream::Unix(s.try_clone()?)),
        }
    }
}

impl Read for RemoteStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            RemoteStream::Tcp(s) => s.read(buf),
            #[cfg(unix)]
            RemoteStream::Unix(s) => s.read(buf),
        }
    }
}

impl Write for RemoteStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            RemoteStream::Tcp(s) => s.write(buf),
            #[cfg(unix)]
            RemoteStream::Unix(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            RemoteStream::Tcp(s) => s.flush(),
            #[cfg(unix)]
            RemoteStream::Unix(s) => s.flush(),
        }
    }
}

/// Full-duplex pipelined client (`tcp://host:port` or `unix:///path/to.sock`) for `rust/rl/infer_server.py`.
pub struct RemoteNet {
    pub name: String,
    queue: Arc<(Mutex<BatchQueue>, Condvar)>,
    has_wait: bool,
    buckets: usize,
    emb: Vec<f32>,
    dim: usize,
    trunk: usize,
    belief: Option<Linear>,
    threat: Option<Linear>,
}

impl Drop for RemoteNet {
    fn drop(&mut self) {
        let (lock, cv) = &*self.queue;
        if let Ok(mut g) = lock.lock() {
            g.closed = true;
            cv.notify_all();
        }
    }
}

impl std::fmt::Debug for RemoteNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteNet").field("name", &self.name).field("dim", &self.dim).finish()
    }
}

fn read_u32(s: &mut impl Read) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    s.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_f32s(s: &mut impl Read, n: usize) -> std::io::Result<Vec<f32>> {
    let mut bytes = vec![0u8; n * 4];
    s.read_exact(&mut bytes)?;
    Ok(bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

#[inline]
fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exp = ((bits >> 10) & 0x1f) as i32;
    let mant = (bits & 0x03ff) as u32;
    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(sign);
        }
        let mut m = mant;
        let mut e = -14i32;
        while (m & 0x0400) == 0 {
            m <<= 1;
            e -= 1;
        }
        m &= 0x03ff;
        let f_exp = ((e + 127) as u32) << 23;
        return f32::from_bits(sign | f_exp | (m << 13));
    }
    if exp == 31 {
        return f32::from_bits(sign | 0x7f80_0000 | (mant << 13));
    }
    let f_exp = ((exp - 15 + 127) as u32) << 23;
    f32::from_bits(sign | f_exp | (mant << 13))
}

fn complete_slot(slot: &BatchSlot, idx: usize, out: Output) {
    let (rlock, rcv) = &**slot;
    let mut st = rlock.lock().unwrap();
    if idx < st.outs.len() {
        st.outs[idx] = out;
    }
    st.rem = st.rem.saturating_sub(1);
    let done = st.rem == 0;
    drop(st);
    if done {
        rcv.notify_one();
    }
}

fn fail_reply(slot: &BatchSlot, idx: usize, n_acts: usize, need_h: bool, trunk: usize) {
    complete_slot(
        slot,
        idx,
        Output {
            logits: vec![0.0; n_acts],
            value: 0.0,
            h: if need_h { vec![0.0; trunk] } else { Vec::new() },
            act_q: Vec::new(),
        },
    );
}

impl RemoteNet {
    pub fn connect(url: &str) -> Result<RemoteNet, String> {
        let mut stream = if let Some(addr) = url.strip_prefix("tcp://") {
            let s = TcpStream::connect(addr).map_err(|e| format!("{url}: {e}"))?;
            s.set_nodelay(true).ok();
            RemoteStream::Tcp(s)
        } else if let Some(sock_path) = url.strip_prefix("unix://").or_else(|| url.strip_prefix("unix:")) {
            #[cfg(unix)]
            {
                let s = std::os::unix::net::UnixStream::connect(sock_path).map_err(|e| format!("{url}: {e}"))?;
                RemoteStream::Unix(s)
            }
            #[cfg(not(unix))]
            {
                let _ = sock_path;
                return Err(format!("{url}: unix domain sockets require a Unix OS"));
            }
        } else {
            return Err("expected tcp://host:port or unix:///path/to.sock".to_string());
        };

        let dim = read_u32(&mut stream).map_err(|e| format!("{url} handshake: {e}"))? as usize;
        let buckets = read_u32(&mut stream).map_err(|e| format!("{url} handshake: {e}"))? as usize;
        let trunk = read_u32(&mut stream).map_err(|e| format!("{url} handshake: {e}"))? as usize;
        let mut has_b = [0u8; 1];
        stream.read_exact(&mut has_b).map_err(|e| format!("{url} handshake: {e}"))?;
        let proto_v2 = (has_b[0] & 0x80) != 0;
        let has_act_q = proto_v2 && (has_b[0] & 4) != 0;
        let (emb, belief) = if (has_b[0] & 1) != 0 {
            let emb = read_f32s(&mut stream, buckets * dim).map_err(|e| format!("{url} emb: {e}"))?;
            let bw = read_f32s(&mut stream, dim * trunk).map_err(|e| format!("{url} bw: {e}"))?;
            let bb = read_f32s(&mut stream, dim).map_err(|e| format!("{url} bb: {e}"))?;
            (emb, Some(Linear::new(bw, bb, dim, trunk)))
        } else {
            (Vec::new(), None)
        };
        let threat = if (has_b[0] & 2) != 0 {
            let tf = crate::obs::BELIEF_META_F;
            let tw = read_f32s(&mut stream, tf * trunk).map_err(|e| format!("{url} tw: {e}"))?;
            let tb = read_f32s(&mut stream, tf).map_err(|e| format!("{url} tb: {e}"))?;
            Some(Linear::new(tw, tb, tf, trunk))
        } else {
            None
        };

        let mut read_stream = stream.try_clone().map_err(|e| format!("{url} clone: {e}"))?;
        let queue = Arc::new((Mutex::new(BatchQueue { reqs: Vec::new(), closed: false }), Condvar::new()));
        let inflight: Arc<Mutex<HashMap<u32, InFlightBatch>>> = Arc::new(Mutex::new(HashMap::new()));
        let max_batch: usize =
            std::env::var("PLAYMAT_GPU_BATCH").ok().and_then(|v| v.parse().ok()).unwrap_or(512).max(1);
        let wait_us: u64 = std::env::var("PLAYMAT_GPU_WAIT_US").ok().and_then(|v| v.parse().ok()).unwrap_or(0);

        // Reader thread: receives completed GPU batches out of the full-duplex socket.
        let q_reader = Arc::clone(&queue);
        let inf_reader = Arc::clone(&inflight);
        std::thread::spawn(move || {
            let mut resp_buf = Vec::<u8>::with_capacity(1 << 18);
            loop {
                let (req_id, rb, rtrunk) = if proto_v2 {
                    let Ok(id) = read_u32(&mut read_stream) else { break };
                    let Ok(b) = read_u32(&mut read_stream) else { break };
                    let Ok(rt) = read_u32(&mut read_stream) else { break };
                    (id, b as usize, rt as usize)
                } else {
                    let Ok(b) = read_u32(&mut read_stream) else { break };
                    let Ok(rt) = read_u32(&mut read_stream) else { break };
                    (1u32, b as usize, rt as usize)
                };
                let entry = inf_reader.lock().unwrap().remove(&req_id);
                let Some(batch) = entry else { break };
                let b = batch.items.len();
                debug_assert_eq!(rb, b);
                let w_a = batch.w_a;
                let v_bytes = b * 4;
                let l_bytes = b * w_a * 4;
                let aq_bytes = if has_act_q { b * w_a * 2 } else { 0 };
                let h_bytes = b * rtrunk * 4;
                let need = v_bytes + l_bytes + aq_bytes + h_bytes;
                resp_buf.resize(need, 0);
                if read_stream.read_exact(&mut resp_buf).is_err() {
                    for it in batch.items {
                        fail_reply(&it.slot, it.idx, it.n_acts, it.need_h, trunk);
                    }
                    break;
                }
                let vs = resp_buf[..v_bytes].as_chunks::<4>().0;
                let all_logits = resp_buf[v_bytes..v_bytes + l_bytes].as_chunks::<4>().0;
                let all_aq = resp_buf[v_bytes + l_bytes..v_bytes + l_bytes + aq_bytes].as_chunks::<2>().0;
                let all_h = resp_buf[v_bytes + l_bytes + aq_bytes..].as_chunks::<4>().0;
                for (i, it) in batch.items.into_iter().enumerate() {
                    let na = it.n_acts.min(w_a);
                    let mut logits = Vec::with_capacity(na);
                    for c in &all_logits[i * w_a..i * w_a + na] {
                        logits.push(f32::from_le_bytes(*c));
                    }
                    let act_q = if has_act_q {
                        let mut qv = Vec::with_capacity(na);
                        for c in &all_aq[i * w_a..i * w_a + na] {
                            qv.push(f16_to_f32(u16::from_le_bytes(*c)));
                        }
                        qv
                    } else {
                        Vec::new()
                    };
                    let h = if it.need_h && rtrunk > 0 {
                        let mut hv = Vec::with_capacity(rtrunk);
                        for c in &all_h[i * rtrunk..(i + 1) * rtrunk] {
                            hv.push(f32::from_le_bytes(*c));
                        }
                        hv
                    } else {
                        Vec::new()
                    };
                    let value = f32::from_le_bytes(vs[i]);
                    complete_slot(&it.slot, it.idx, Output { logits, value, h, act_q });
                }
            }
            // Connection closed: mark queue closed and wake any in-flight or queued callers so nobody deadlocks.
            let (lock, cv) = &*q_reader;
            let leftover_q = {
                let mut g = lock.lock().unwrap();
                g.closed = true;
                cv.notify_all();
                std::mem::take(&mut g.reqs)
            };
            for r in leftover_q {
                fail_reply(&r.slot, r.idx, r.obs.n_acts(), r.need_h, trunk);
            }
            let leftover_inf: Vec<InFlightBatch> =
                inf_reader.lock().unwrap().drain().map(|(_, v)| v).collect();
            for b in leftover_inf {
                for it in b.items {
                    fail_reply(&it.slot, it.idx, it.n_acts, it.need_h, trunk);
                }
            }
        });

        // Writer thread: drains up to `max_batch = 512` concurrent Rayon thread items at once and streams immediately
        // over the full-duplex socket so `infer_server.py` coalesces all concurrent waves into B=128..512 superbatches.
        let q_writer = Arc::clone(&queue);
        let inf_writer = Arc::clone(&inflight);
        std::thread::spawn(move || {
            let mut payload = Vec::<u8>::with_capacity(1 << 20);
            let mut next_req_id = 1u32;
            loop {
                let batch = {
                    let (lock, cv) = &*q_writer;
                    let mut g = lock.lock().unwrap();
                    while g.reqs.is_empty() && !g.closed {
                        g = cv.wait(g).unwrap();
                    }
                    if g.reqs.is_empty() && g.closed {
                        let eof_hdr = if proto_v2 { &[0u8; 16][..] } else { &[0u8; 12][..] };
                        let _ = stream.write_all(eof_hdr);
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
                let w_e = (max_e.div_ceil(8) * 8).max(8);
                let w_a = (max_a.div_ceil(4) * 4).max(4);
                let need_h = batch.iter().any(|r| r.need_h);
                let req_id = if proto_v2 {
                    let id = next_req_id;
                    next_req_id = next_req_id.wrapping_add(1);
                    if next_req_id == 0 {
                        next_req_id = 1;
                    }
                    id
                } else {
                    1u32
                };

                let elem_bytes = if proto_v2 { 2 } else { 4 };
                let hdr_bytes = if proto_v2 { 16 } else { 12 };
                let glob_bytes = b * GLOB_F * elem_bytes;
                let eids_bytes = b * w_e * 2;
                let ents_bytes = b * w_e * ENT_F * elem_bytes;
                let em_bytes = b * w_e;
                let eridx_bytes = b * w_e;
                let ertyp_bytes = b * w_e;
                let aids_bytes = b * w_a * 2;
                let acts_bytes = b * w_a * ACT_F * elem_bytes;
                let am_bytes = b * w_a;
                let aent_bytes = b * w_a;
                let total_bytes = hdr_bytes
                    + glob_bytes
                    + eids_bytes
                    + ents_bytes
                    + em_bytes
                    + eridx_bytes
                    + ertyp_bytes
                    + aids_bytes
                    + acts_bytes
                    + am_bytes
                    + aent_bytes;
                payload.clear();
                payload.resize(total_bytes, 0);
                if proto_v2 {
                    payload[0..4].copy_from_slice(&req_id.to_le_bytes());
                    payload[4..8].copy_from_slice(&(b as u32).to_le_bytes());
                    payload[8..10].copy_from_slice(&(w_e as u16).to_le_bytes());
                    payload[10..12].copy_from_slice(&(w_a as u16).to_le_bytes());
                    payload[12] = u8::from(need_h);
                } else {
                    payload[0..4].copy_from_slice(&(b as u32).to_le_bytes());
                    payload[4..8].copy_from_slice(&(w_e as u32).to_le_bytes());
                    payload[8..12].copy_from_slice(&(w_a as u32).to_le_bytes());
                }

                let o_glob = hdr_bytes;
                let o_eids = o_glob + glob_bytes;
                let o_ents = o_eids + eids_bytes;
                let o_em = o_ents + ents_bytes;
                let o_eridx = o_em + em_bytes;
                let o_ertyp = o_eridx + eridx_bytes;
                let o_aids = o_ertyp + ertyp_bytes;
                let o_acts = o_aids + aids_bytes;
                let o_am = o_acts + acts_bytes;
                let o_aent = o_am + am_bytes;
                payload[o_eridx..o_eridx + eridx_bytes].fill(NO_ENT);
                payload[o_aent..o_aent + aent_bytes].fill(NO_ENT);

                let mut items = Vec::with_capacity(b);
                for (i, req) in batch.into_iter().enumerate() {
                    let o = &req.obs;
                    if proto_v2 {
                        let gd = &mut payload[o_glob + i * GLOB_F * 2..o_glob + (i + 1) * GLOB_F * 2];
                        for (k, &v) in o.glob.iter().take(GLOB_F).enumerate() {
                            gd[k * 2..(k + 1) * 2].copy_from_slice(&crate::selfplay::f16(v).to_le_bytes());
                        }
                    } else {
                        let gd = &mut payload[o_glob + i * GLOB_F * 4..o_glob + (i + 1) * GLOB_F * 4];
                        for (k, &v) in o.glob.iter().take(GLOB_F).enumerate() {
                            gd[k * 4..(k + 1) * 4].copy_from_slice(&v.to_le_bytes());
                        }
                    }
                    let ne = o.n_ents().min(w_e);
                    if ne > 0 {
                        let id_base = o_eids + i * w_e * 2;
                        for k in 0..ne {
                            payload[id_base + k * 2..id_base + k * 2 + 2]
                                .copy_from_slice(&(o.ent_ids[k] as u16).to_le_bytes());
                        }
                        if proto_v2 {
                            let ef_base = o_ents + i * w_e * ENT_F * 2;
                            for (k, &v) in o.ents[..ne * ENT_F].iter().enumerate() {
                                payload[ef_base + k * 2..ef_base + k * 2 + 2]
                                    .copy_from_slice(&crate::selfplay::f16(v).to_le_bytes());
                            }
                        } else {
                            let ef_base = o_ents + i * w_e * ENT_F * 4;
                            for (k, &v) in o.ents[..ne * ENT_F].iter().enumerate() {
                                payload[ef_base + k * 4..ef_base + k * 4 + 4].copy_from_slice(&v.to_le_bytes());
                            }
                        }
                        let em_base = o_em + i * w_e;
                        payload[em_base..em_base + ne].fill(1);
                        if o.ent_rel_idx.len() >= ne && o.ent_rel_type.len() >= ne {
                            payload[o_eridx + i * w_e..o_eridx + i * w_e + ne].copy_from_slice(&o.ent_rel_idx[..ne]);
                            payload[o_ertyp + i * w_e..o_ertyp + i * w_e + ne].copy_from_slice(&o.ent_rel_type[..ne]);
                        }
                    }
                    let na = o.n_acts().min(w_a);
                    if na > 0 {
                        let id_base = o_aids + i * w_a * 2;
                        for k in 0..na {
                            payload[id_base + k * 2..id_base + k * 2 + 2]
                                .copy_from_slice(&(o.act_ids[k] as u16).to_le_bytes());
                        }
                        if proto_v2 {
                            let af_base = o_acts + i * w_a * ACT_F * 2;
                            for (k, &v) in o.acts[..na * ACT_F].iter().enumerate() {
                                payload[af_base + k * 2..af_base + k * 2 + 2]
                                    .copy_from_slice(&crate::selfplay::f16(v).to_le_bytes());
                            }
                        } else {
                            let af_base = o_acts + i * w_a * ACT_F * 4;
                            for (k, &v) in o.acts[..na * ACT_F].iter().enumerate() {
                                payload[af_base + k * 4..af_base + k * 4 + 4].copy_from_slice(&v.to_le_bytes());
                            }
                        }
                        let am_base = o_am + i * w_a;
                        payload[am_base..am_base + na].fill(1);
                        if o.act_ent_idx.len() >= na {
                            payload[o_aent + i * w_a..o_aent + i * w_a + na].copy_from_slice(&o.act_ent_idx[..na]);
                        }
                    }
                    items.push(InFlightItem { n_acts: o.n_acts(), need_h: req.need_h, slot: req.slot, idx: req.idx });
                }

                inf_writer.lock().unwrap().insert(req_id, InFlightBatch { w_a, items });
                if stream.write_all(&payload).is_err() {
                    if let Some(failed_b) = inf_writer.lock().unwrap().remove(&req_id) {
                        for it in failed_b.items {
                            fail_reply(&it.slot, it.idx, it.n_acts, it.need_h, trunk);
                        }
                    }
                    break;
                }
            }
        });

        Ok(RemoteNet {
            name: url.to_string(),
            queue,
            has_wait: wait_us > 0,
            buckets: buckets.max(obs::BUCKETS as usize),
            emb,
            dim,
            trunk,
            belief,
            threat,
        })
    }

    fn submit_one(&self, o: &Obs, need_h: bool) -> Output {
        let obs_cloned = o.clone();
        let slot: BatchSlot =
            Arc::new((Mutex::new(BatchReplyState { rem: 1, outs: vec![Output::default()] }), Condvar::new()));
        {
            let (lock, cv) = &*self.queue;
            let mut g = lock.lock().unwrap();
            if g.closed {
                return Output {
                    logits: vec![0.0; o.n_acts()],
                    value: 0.0,
                    h: if need_h { vec![0.0; self.trunk] } else { Vec::new() },
                    act_q: Vec::new(),
                };
            }
            let notify = g.reqs.is_empty() || self.has_wait;
            g.reqs.push(PendingReq { obs: obs_cloned, need_h, slot: Arc::clone(&slot), idx: 0 });
            if notify {
                cv.notify_one();
            }
        }
        let (rlock, rcv) = &*slot;
        let mut st = rlock.lock().unwrap();
        while st.rem > 0 {
            st = rcv.wait(st).unwrap();
        }
        st.outs.pop().unwrap_or_default()
    }

    pub fn forward(&self, o: &Obs) -> Output {
        self.submit_one(o, true)
    }

    /// Evaluates one observation with `need_h = false` (omits `trunk` state vector `h` on the wire).
    pub fn forward_no_h(&self, o: &Obs) -> Output {
        self.submit_one(o, false)
    }

    /// Clones observations outside the queue mutex and appends the wave in O(1) under one shared `BatchSlot`
    /// so concurrent Rayon threads never serialize on heap allocations or individual per-item Condvar wakeups.
    pub fn forward_batch(&self, obs: &[Obs]) -> Vec<Output> {
        if obs.is_empty() {
            return Vec::new();
        }
        if obs.len() == 1 {
            return vec![self.submit_one(&obs[0], false)];
        }
        let n = obs.len();
        let slot: BatchSlot =
            Arc::new((Mutex::new(BatchReplyState { rem: n, outs: vec![Output::default(); n] }), Condvar::new()));
        let mut local_reqs: Vec<PendingReq> = obs
            .iter()
            .enumerate()
            .map(|(i, o)| PendingReq { obs: o.clone(), need_h: false, slot: Arc::clone(&slot), idx: i })
            .collect();
        {
            let (lock, cv) = &*self.queue;
            let mut g = lock.lock().unwrap();
            if g.closed {
                return obs
                    .iter()
                    .map(|o| Output { logits: vec![0.0; o.n_acts()], value: 0.0, h: Vec::new(), act_q: Vec::new() })
                    .collect();
            }
            let notify = g.reqs.is_empty() || self.has_wait;
            g.reqs.append(&mut local_reqs);
            if notify {
                cv.notify_one();
            }
        }
        let (rlock, rcv) = &*slot;
        let mut st = rlock.lock().unwrap();
        while st.rem > 0 {
            st = rcv.wait(st).unwrap();
        }
        std::mem::take(&mut st.outs)
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

    pub fn threat(&self, h: &[f32]) -> [f32; crate::obs::BELIEF_META_F] {
        let mut out = [0.0f32; crate::obs::BELIEF_META_F];
        if let Some(th) = &self.threat {
            let mut q = Vec::new();
            th.apply_act(h, &mut q, false, false);
            let n = out.len().min(q.len());
            out[..n].copy_from_slice(&q[..n]);
        }
        out
    }
}

/// A network: the first, pooled one, the transformer, or a GPU-batched remote server (`tcp://...`)
// `Tx` is large, but a game holds one `Arc<Net>`, so boxing it buys nothing
#[allow(clippy::large_enum_variant)]
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

    /// Card-embedding vocabulary size (`4096` for Phase 5/6 `.pmnet` & servers, `32768` for Phase 7 `TxV2` servers)
    #[inline]
    pub fn buckets(&self) -> usize {
        match self {
            Net::Mlp(n) => (n.emb.len() / n.dim.max(1)).max(obs::BUCKETS as usize),
            Net::Tx(n) => (n.emb.len() / n.dim.max(1)).max(obs::BUCKETS as usize),
            Net::Remote(n) => n.buckets.max(obs::BUCKETS as usize),
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

    /// Batched forward evaluation: coalesces an entire virtual-loss wave into one atomic GPU RPC on RemoteNet.
    pub fn forward_batch(&self, obs: &[Obs]) -> Vec<Output> {
        match self {
            Net::Remote(n) => n.forward_batch(obs),
            Net::Tx(n) => obs.iter().map(|o| n.forward(o)).collect(),
            Net::Mlp(n) => obs.iter().map(|o| n.forward(o)).collect(),
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

    pub fn has_threat(&self) -> bool {
        match self {
            Net::Mlp(_) => false,
            Net::Tx(n) => n.threat.is_some(),
            Net::Remote(n) => n.threat.is_some(),
        }
    }

    pub fn threat(&self, h: &[f32]) -> [f32; crate::obs::BELIEF_META_F] {
        match self {
            Net::Mlp(_) => [0.0; crate::obs::BELIEF_META_F],
            Net::Tx(n) => n.threat(h),
            Net::Remote(n) => n.threat(h),
        }
    }
}

/// Softmax of `logits / temp`
pub fn softmax(logits: &[f32], temp: f32) -> Vec<f32> {
    let t = temp.max(1e-3);
    let m = logits.iter().copied().fold(f32::MIN, f32::max);
    let e: Vec<f32> = logits.iter().map(|l| ((l - m) / t).sexp()).collect();
    let s: f32 = e.iter().sum();
    e.iter().map(|v| v / s).collect()
}

static CACHE: OnceLock<Mutex<HashMap<String, Arc<Net>>>> = OnceLock::new();

/// Registers raw `.pmnet` bytes in the process network cache under `key` (used by WASM / in-memory workers).
pub fn register_bytes(key: &str, bytes: &[u8]) -> Result<Arc<Net>, String> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap();
    if let Some(n) = map.get(key) {
        return Ok(Arc::clone(n));
    }
    let net = Arc::new(Net::parse(key, bytes)?);
    map.insert(key.to_string(), Arc::clone(&net));
    Ok(net)
}

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
    } else if path.starts_with("tcp://") || path.starts_with("unix://") || path.starts_with("unix:") {
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
    let o = obs::observe_for(game, seat, &q, &[], net.buckets());
    (net.state(&o).1 as f64 * 1000.0).clamp(-30_000.0, 30_000.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn serve_v2_once(mut stream: impl Read + Write + Send + 'static, local: Arc<Net>) {
        std::thread::spawn(move || {
            // Build protocol v2 handshake matching infer_server.py::pack_handshake
            let (dim, trunk, emb, bw, bb, tw_opt) = match &*local {
                Net::Tx(tx) => (
                    tx.dim as u32,
                    tx.trunk1.out as u32,
                    tx.emb.clone(),
                    tx.belief.w.clone(),
                    tx.belief.b.clone(),
                    tx.threat.as_ref().map(|th| (th.w.clone(), th.b.clone())),
                ),
                Net::Mlp(mlp) => {
                    let bl = mlp.belief.as_ref().unwrap();
                    (mlp.dim as u32, mlp.trunk2.out as u32, mlp.emb.clone(), bl.w.clone(), bl.b.clone(), None)
                }
                Net::Remote(_) => unreachable!(),
            };
            let flags: u8 = 1 | (if tw_opt.is_some() { 2 } else { 0 }) | 0x80;
            let mut hs = Vec::new();
            hs.extend_from_slice(&dim.to_le_bytes());
            hs.extend_from_slice(&obs::BUCKETS.to_le_bytes());
            hs.extend_from_slice(&trunk.to_le_bytes());
            hs.push(flags);
            for x in emb.iter().chain(&bw).chain(&bb) {
                hs.extend_from_slice(&x.to_le_bytes());
            }
            if let Some((tw, tb)) = tw_opt {
                for x in tw.iter().chain(&tb) {
                    hs.extend_from_slice(&x.to_le_bytes());
                }
            }
            if stream.write_all(&hs).is_err() {
                return;
            }
            loop {
                let mut hdr = [0u8; 16];
                if stream.read_exact(&mut hdr).is_err() {
                    break;
                }
                let req_id = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
                let b = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
                if b == 0 {
                    break;
                }
                let w_e = u16::from_le_bytes(hdr[8..10].try_into().unwrap()) as usize;
                let w_a = u16::from_le_bytes(hdr[10..12].try_into().unwrap()) as usize;
                let need_h = hdr[12] != 0;

                let glob_b = b * GLOB_F * 2;
                let eids_b = b * w_e * 2;
                let ents_b = b * w_e * ENT_F * 2;
                let em_b = b * w_e;
                let eridx_b = b * w_e;
                let ertyp_b = b * w_e;
                let aids_b = b * w_a * 2;
                let acts_b = b * w_a * ACT_F * 2;
                let am_b = b * w_a;
                let aent_b = b * w_a;
                let tot = glob_b + eids_b + ents_b + em_b + eridx_b + ertyp_b + aids_b + acts_b + am_b + aent_b;
                let mut raw = vec![0u8; tot];
                if stream.read_exact(&mut raw).is_err() {
                    break;
                }
                let o_glob = 0;
                let o_eids = o_glob + glob_b;
                let o_ents = o_eids + eids_b;
                let o_em = o_ents + ents_b;
                let o_eridx = o_em + em_b;
                let o_ertyp = o_eridx + eridx_b;
                let o_aids = o_ertyp + ertyp_b;
                let o_acts = o_aids + aids_b;
                let o_am = o_acts + acts_b;
                let o_aent = o_am + am_b;

                let mut vs = Vec::with_capacity(b);
                let mut ls = Vec::with_capacity(b * w_a);
                let mut hs_out = Vec::with_capacity(if need_h { b * trunk as usize } else { 0 });

                for i in 0..b {
                    let mut glob = Vec::with_capacity(GLOB_F);
                    for c in raw[o_glob + i * GLOB_F * 2..o_glob + (i + 1) * GLOB_F * 2].as_chunks::<2>().0 {
                        glob.push(f16_to_f32(u16::from_le_bytes(*c)));
                    }
                    let em_row = &raw[o_em + i * w_e..o_em + (i + 1) * w_e];
                    let ne = em_row.iter().take_while(|&&m| m != 0).count();
                    let mut ent_ids = Vec::with_capacity(ne);
                    for c in raw[o_eids + i * w_e * 2..o_eids + i * w_e * 2 + ne * 2].as_chunks::<2>().0 {
                        ent_ids.push(u16::from_le_bytes(*c) as u32);
                    }
                    let mut ents = Vec::with_capacity(ne * ENT_F);
                    for c in raw[o_ents + i * w_e * ENT_F * 2..o_ents + i * w_e * ENT_F * 2 + ne * ENT_F * 2]
                        .as_chunks::<2>()
                        .0
                    {
                        ents.push(f16_to_f32(u16::from_le_bytes(*c)));
                    }
                    let ent_rel_idx = raw[o_eridx + i * w_e..o_eridx + i * w_e + ne].to_vec();
                    let ent_rel_type = raw[o_ertyp + i * w_e..o_ertyp + i * w_e + ne].to_vec();

                    let am_row = &raw[o_am + i * w_a..o_am + (i + 1) * w_a];
                    let na = am_row.iter().take_while(|&&m| m != 0).count();
                    let mut act_ids = Vec::with_capacity(na);
                    for c in raw[o_aids + i * w_a * 2..o_aids + i * w_a * 2 + na * 2].as_chunks::<2>().0 {
                        act_ids.push(u16::from_le_bytes(*c) as u32);
                    }
                    let mut acts = Vec::with_capacity(na * ACT_F);
                    for c in raw[o_acts + i * w_a * ACT_F * 2..o_acts + i * w_a * ACT_F * 2 + na * ACT_F * 2]
                        .as_chunks::<2>()
                        .0
                    {
                        acts.push(f16_to_f32(u16::from_le_bytes(*c)));
                    }
                    let act_ent_idx = raw[o_aent + i * w_a..o_aent + i * w_a + na].to_vec();

                    let decoded = Obs { glob, ents, ent_ids, ent_rel_idx, ent_rel_type, acts, act_ids, act_ent_idx };
                    let out = local.forward(&decoded);
                    vs.push(out.value);
                    for a_k in 0..w_a {
                        ls.push(out.logits.get(a_k).copied().unwrap_or(0.0));
                    }
                    if need_h {
                        hs_out.extend_from_slice(&out.h);
                    }
                }
                let rtrunk: u32 = if need_h { trunk } else { 0 };
                let mut resp = Vec::with_capacity(12 + (vs.len() + ls.len() + hs_out.len()) * 4);
                resp.extend_from_slice(&req_id.to_le_bytes());
                resp.extend_from_slice(&(b as u32).to_le_bytes());
                resp.extend_from_slice(&rtrunk.to_le_bytes());
                for x in vs.iter().chain(&ls).chain(&hs_out) {
                    resp.extend_from_slice(&x.to_le_bytes());
                }
                if stream.write_all(&resp).is_err() {
                    break;
                }
            }
        });
    }

    #[test]
    fn remote_net_v2_round_trips_need_h_0_and_1_over_tcp_and_unix() {
        let tx_bytes = include_bytes!("../tests/tx_rel_small.pmnet");
        let local = Arc::new(Net::parse("tx_rel_small", tx_bytes).expect("parse tx_rel_small"));
        let obs1 = Obs {
            glob: (0..GLOB_F).map(|i| (i as f32) * 0.03 - 1.0).collect(),
            ents: (0..ENT_F * 3).map(|i| ((i % 7) as f32) * 0.2).collect(),
            ent_ids: vec![10, 250, 1024],
            ent_rel_idx: vec![1, NO_ENT, 0],
            ent_rel_type: vec![2, 0, 1],
            acts: (0..ACT_F * 3).map(|i| ((i % 5) as f32) * 0.25).collect(),
            act_ids: vec![0, 10, 250],
            act_ent_idx: vec![NO_ENT, 0, 1],
        };
        let mut obs2 = obs1.clone();
        obs2.glob[0] = 0.75;
        obs2.acts.truncate(ACT_F * 2);
        obs2.act_ids.truncate(2);
        obs2.act_ent_idx.truncate(2);

        let expected1 = local.forward(&obs1);
        let expected2 = local.forward(&obs2);

        // 1. Test over TCP loopback
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind tcp");
        let addr = listener.local_addr().unwrap();
        let local_tcp = Arc::clone(&local);
        std::thread::spawn(move || {
            if let Ok((s, _)) = listener.accept() {
                serve_v2_once(s, local_tcp);
            }
        });
        let remote_tcp = RemoteNet::connect(&format!("tcp://{addr}")).expect("connect tcp");

        // need_h = 1 (root forward)
        let r1_with_h = remote_tcp.forward(&obs1);
        assert_eq!(r1_with_h.h.len(), expected1.h.len());
        assert!(!r1_with_h.h.is_empty());
        assert!((r1_with_h.value - expected1.value).abs() < 5e-3, "{} vs {}", r1_with_h.value, expected1.value);
        for (a, b) in r1_with_h.logits.iter().zip(&expected1.logits) {
            assert!((a - b).abs() < 5e-3, "logit {a} vs {b}");
        }

        // need_h = 0 (leaf forward_batch & forward_no_h)
        let r_batch = remote_tcp.forward_batch(&[obs1.clone(), obs2.clone()]);
        assert_eq!(r_batch.len(), 2);
        assert!(r_batch[0].h.is_empty(), "forward_batch (need_h=0) must omit h vector");
        assert!(r_batch[1].h.is_empty(), "forward_batch (need_h=0) must omit h vector");
        assert!((r_batch[0].value - r1_with_h.value).abs() < 1e-5);
        assert!((r_batch[1].value - expected2.value).abs() < 5e-3);
        assert_eq!(r_batch[1].logits.len(), 2);

        let r_single_no_h = remote_tcp.forward_no_h(&obs2);
        assert!(r_single_no_h.h.is_empty(), "forward_no_h must omit h vector");
        assert!((r_single_no_h.value - r_batch[1].value).abs() < 1e-5);

        // 2. Test over Unix domain socket + concurrent pipelined callers
        #[cfg(unix)]
        {
            let sock_path = std::env::temp_dir().join(format!("playmat_v2_test_{}.sock", std::process::id()));
            let _ = std::fs::remove_file(&sock_path);
            let ulistener = std::os::unix::net::UnixListener::bind(&sock_path).expect("bind unix");
            let local_unix = Arc::clone(&local);
            std::thread::spawn(move || {
                if let Ok((s, _)) = ulistener.accept() {
                    serve_v2_once(s, local_unix);
                }
            });
            let remote_unix =
                Arc::new(RemoteNet::connect(&format!("unix://{}", sock_path.display())).expect("connect unix"));
            std::thread::scope(|s| {
                for t in 0..4 {
                    let ru = Arc::clone(&remote_unix);
                    let o1 = obs1.clone();
                    let o2 = obs2.clone();
                    let exp_h_len = expected1.h.len();
                    s.spawn(move || {
                        for iter in 0..4 {
                            if (t + iter) % 2 == 0 {
                                let out = ru.forward(&o1);
                                assert_eq!(out.h.len(), exp_h_len);
                            } else {
                                let outs = ru.forward_batch(&[o1.clone(), o2.clone()]);
                                assert!(outs[0].h.is_empty() && outs[1].h.is_empty());
                            }
                        }
                    });
                }
            });
            drop(remote_unix);
            let _ = std::fs::remove_file(&sock_path);
        }
    }

    #[test]
    fn remote_net_v2_supports_32k_buckets_u16_ids_and_act_q() {
        let buckets: u32 = crate::obs::BUCKETS_32K as u32;
        let dim: u32 = 256;
        let trunk: u32 = 64;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind tcp 32k");
        let addr = listener.local_addr().unwrap();

        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else { return };
            // Handshake flags: 0x01 (belief) | 0x02 (threat) | 0x04 (act_q) | 0x80 (proto v2)
            let flags: u8 = 0x87;
            let mut hs = Vec::with_capacity(13 + ((buckets * dim + dim * trunk + dim + 8 * trunk + 8) as usize) * 4);
            hs.extend_from_slice(&dim.to_le_bytes());
            hs.extend_from_slice(&buckets.to_le_bytes());
            hs.extend_from_slice(&trunk.to_le_bytes());
            hs.push(flags);
            // Distinct embedding for bucket 31_000 and 32_767 to verify large 33.5 MB table + belief dot product
            let mut emb = vec![0.0f32; (buckets * dim) as usize];
            emb[31_000 * dim as usize] = 2.5;
            emb[32_767 * dim as usize + 1] = -1.5;
            let mut bw = vec![0.0f32; (dim * trunk) as usize];
            bw[0] = 1.0; // dim 0 reads h[0]
            bw[trunk as usize + 1] = 1.0; // dim 1 reads h[1]
            let bb = vec![0.25f32; dim as usize];
            let tw = vec![0.0f32; (8 * trunk) as usize];
            let tb = vec![0.5f32; 8];
            for v in emb.iter().chain(&bw).chain(&bb).chain(&tw).chain(&tb) {
                hs.extend_from_slice(&v.to_le_bytes());
            }
            if stream.write_all(&hs).is_err() {
                return;
            }

            let mut hdr = [0u8; 16];
            while stream.read_exact(&mut hdr).is_ok() {
                let req_id = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
                let b = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;
                let w_e = u16::from_le_bytes(hdr[8..10].try_into().unwrap()) as usize;
                let w_a = u16::from_le_bytes(hdr[10..12].try_into().unwrap()) as usize;
                let need_h = hdr[12] != 0;
                if b == 0 {
                    break;
                }
                let payload_len = b * GLOB_F * 2 + b * w_e * 2 + b * w_e * ENT_F * 2 + b * w_e * 3 + b * w_a * 2 + b * w_a * ACT_F * 2 + b * w_a * 2;
                let mut buf = vec![0u8; payload_len];
                if stream.read_exact(&mut buf).is_err() {
                    break;
                }
                let eids_off = b * GLOB_F * 2;
                let aids_off = eids_off + b * w_e * 2 + b * w_e * ENT_F * 2 + b * w_e * 3;
                // Verify unsigned u16 IDs >= 32768 arrive intact without signed i16 wrap
                let e0 = u16::from_le_bytes(buf[eids_off..eids_off + 2].try_into().unwrap());
                let e1 = u16::from_le_bytes(buf[eids_off + 2..eids_off + 4].try_into().unwrap());
                let a1 = u16::from_le_bytes(buf[aids_off + 2..aids_off + 4].try_into().unwrap());
                assert_eq!(e0, 32_767);
                assert_eq!(e1, 45_000);
                assert_eq!(a1, 32_768);

                let rtrunk: u32 = if need_h { trunk } else { 0 };
                let mut resp = Vec::new();
                resp.extend_from_slice(&req_id.to_le_bytes());
                resp.extend_from_slice(&(b as u32).to_le_bytes());
                resp.extend_from_slice(&rtrunk.to_le_bytes());
                for bi in 0..b {
                    let val = 0.5f32 + bi as f32 * 0.25;
                    resp.extend_from_slice(&val.to_le_bytes());
                }
                for bi in 0..b {
                    for ak in 0..w_a {
                        let lg = (bi as f32) - (ak as f32) * 0.5;
                        resp.extend_from_slice(&lg.to_le_bytes());
                    }
                }
                for bi in 0..b {
                    for ak in 0..w_a {
                        let q = (0.15f32 + 0.35f32 * ak as f32 + 0.05f32 * bi as f32).clamp(0.01, 0.99);
                        resp.extend_from_slice(&crate::selfplay::f16(q).to_le_bytes());
                    }
                }
                if need_h {
                    for _ in 0..b {
                        let mut h = vec![0.0f32; trunk as usize];
                        h[0] = 2.0;
                        h[1] = 3.0;
                        for x in h {
                            resp.extend_from_slice(&x.to_le_bytes());
                        }
                    }
                }
                if stream.write_all(&resp).is_err() {
                    break;
                }
            }
        });

        let net = Net::Remote(Arc::new(RemoteNet::connect(&format!("tcp://{addr}")).expect("connect 32k")));
        assert_eq!(net.buckets(), crate::obs::BUCKETS_32K);

        let obs = Obs {
            glob: vec![0.25; GLOB_F],
            ents: vec![0.1; ENT_F * 2],
            ent_ids: vec![32_767, 45_000],
            ent_rel_idx: vec![NO_ENT, 0],
            ent_rel_type: vec![0, 1],
            acts: vec![0.2; ACT_F * 3],
            act_ids: vec![0, 32_768, 31_000],
            act_ent_idx: vec![NO_ENT, 0, 1],
        };

        // 1. Root forward (`need_h = 1`) + 32k belief check
        let out_h = net.forward(&obs);
        assert_eq!(out_h.h.len(), trunk as usize);
        assert_eq!(out_h.act_q.len(), 3);
        assert!((out_h.act_q[0] - 0.15).abs() < 2e-3);
        assert!((out_h.act_q[1] - 0.50).abs() < 2e-3);
        assert!((out_h.act_q[2] - 0.85).abs() < 2e-3);
        let bl = net.belief(&out_h.h, &[31_000, 32_767]);
        // q[0] = 1*2.0 + 0.25 = 2.25 => emb[31_000, 0] * q[0] = 2.5 * 2.25 = 5.625
        // q[1] = 1*3.0 + 0.25 = 3.25 => emb[32_767, 1] * q[1] = -1.5 * 3.25 = -4.875
        assert!((bl[0] - 5.625).abs() < 1e-4, "got {}", bl[0]);
        assert!((bl[1] - (-4.875)).abs() < 1e-4, "got {}", bl[1]);

        // 2. Leaf batch forward (`need_h = 0`)
        let outs_no_h = net.forward_batch(&[obs.clone(), obs]);
        assert_eq!(outs_no_h.len(), 2);
        assert!(outs_no_h[0].h.is_empty() && outs_no_h[1].h.is_empty());
        assert_eq!(outs_no_h[0].act_q.len(), 3);
        assert_eq!(outs_no_h[1].act_q.len(), 3);
        assert!((outs_no_h[1].act_q[2] - 0.90).abs() < 2e-3);
    }
}
