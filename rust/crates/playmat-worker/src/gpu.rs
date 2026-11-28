//! S5 GPU inference sidecar management and CPU-vs-GPU parity checks (`docs/swarm/S5.md`).
//!
//! For `UnitKind::SelfplayGpu` units, `playmat-worker` downloads and SHA-256 checks each `.pmnet` as usual, spawns
//! `rust/rl/infer_server.py --pmnet <path> --port 0` on loopback, and swaps `net=@<sha256>` -> `net=tcp://127.0.0.1:<port>`
//! so `playmat-ai`'s `RemoteNet` batches concurrent virtual-loss waves onto the GPU while units stay pure data.

use playmat_ai::net::Net;
use playmat_ai::obs::{Obs, ACT_F, BUCKETS, ENT_F, GLOB_F, NO_ENT};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

pub struct InferSidecar {
    pub url: String,
    child: Child,
}

impl InferSidecar {
    /// Spawns `infer_server.py --pmnet <pmnet> --port 0` and waits for its `INFER_SERVER ready tcp://...` line.
    pub fn spawn(python: &str, repo: &str, pmnet: &str, device: &str, amp: bool) -> Result<Self, String> {
        let script = format!("{repo}/rust/rl/infer_server.py");
        let mut child = Command::new(python)
            .arg(&script)
            .arg("--pmnet")
            .arg(pmnet)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg("0")
            .arg("--device")
            .arg(device)
            .arg("--amp")
            .arg(if amp { "1" } else { "0" })
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("spawn {python} {script}: {e}"))?;

        let stdout = child.stdout.take().ok_or("infer_server stdout not piped")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let url = loop {
            line.clear();
            let n = reader.read_line(&mut line).map_err(|e| format!("infer_server stdout: {e}"))?;
            if n == 0 {
                let _ = child.wait();
                return Err("infer_server exited before reporting ready".into());
            }
            if let Some(rest) = line.trim().strip_prefix("INFER_SERVER ready ") {
                if let Some(addr) = rest.split_whitespace().next() {
                    break addr.to_string();
                }
            }
        };
        // Drain remaining stdout on a background thread so the sidecar never blocks on a full pipe.
        std::thread::spawn(move || {
            let mut sink = String::new();
            while reader.read_line(&mut sink).unwrap_or(0) > 0 {
                sink.clear();
            }
        });
        Ok(Self { url, child })
    }
}

impl Drop for InferSidecar {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Detects an available GPU label (`cuda:<name>` via `nvidia-smi`, or `mps` on Apple Silicon).
pub fn detect_gpu() -> Option<String> {
    if let Ok(out) = Command::new("nvidia-smi").args(["--query-gpu=name", "--format=csv,noheader"]).output() {
        if out.status.success() {
            let name = String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_string();
            if !name.is_empty() {
                return Some(format!("cuda:{name}"));
            }
        }
    }
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Some("mps:apple-silicon".into());
    }
    None
}

pub struct ParityReport {
    pub cases: usize,
    pub max_logit_diff: f32,
    pub max_value_diff: f32,
}

fn lcg(state: &mut u64) -> f32 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let bits = (*state >> 40) as u32;
    (bits as f32) / ((1u32 << 24) as f32)
}

fn synthetic_obs(seed: u64, n_ents: usize, n_acts: usize) -> Obs {
    let mut s = seed;
    let glob = (0..GLOB_F).map(|_| lcg(&mut s)).collect();
    let ents = (0..n_ents * ENT_F).map(|_| lcg(&mut s)).collect();
    let ent_ids = (0..n_ents).map(|_| 1 + ((lcg(&mut s) * (BUCKETS - 1) as f32) as u32) % (BUCKETS - 1)).collect();
    let ent_rel_idx = vec![NO_ENT; n_ents];
    let ent_rel_type = vec![0u8; n_ents];
    let acts = (0..n_acts * ACT_F).map(|_| lcg(&mut s)).collect();
    let act_ids = (0..n_acts).map(|_| 1 + ((lcg(&mut s) * (BUCKETS - 1) as f32) as u32) % (BUCKETS - 1)).collect();
    let act_ent_idx = (0..n_acts).map(|i| if n_ents > 0 { (i % n_ents) as u8 } else { NO_ENT }).collect();
    Obs { glob, ents, ent_ids, ent_rel_idx, ent_rel_type, acts, act_ids, act_ent_idx }
}

/// Evaluates `cases` deterministic observations through both CPU `Net::parse` and `tcp://...` GPU `RemoteNet`
/// (in one batch) and checks that every logit and value agrees within `tol`.
pub fn check_parity(pmnet: &str, infer_url: &str, cases: usize, tol: f32) -> Result<ParityReport, String> {
    let bytes = std::fs::read(pmnet).map_err(|e| format!("{pmnet}: {e}"))?;
    let cpu = Net::parse(pmnet, &bytes)?;
    let gpu = playmat_ai::net::load(infer_url)?;

    let shapes = [(1, 2), (4, 3), (12, 6), (24, 10)];
    let obs: Vec<Obs> = (0..cases.max(1))
        .map(|i| {
            let (ne, na) = shapes[i % shapes.len()];
            synthetic_obs(0x55_0000 + i as u64, ne, na)
        })
        .collect();

    let gpu_outs = gpu.forward_batch(&obs);
    let mut max_logit_diff = 0.0f32;
    let mut max_value_diff = 0.0f32;
    for (o, go) in obs.iter().zip(gpu_outs.iter()) {
        let co = cpu.forward(o);
        let vd = (co.value - go.value).abs();
        if vd > max_value_diff {
            max_value_diff = vd;
        }
        if co.logits.len() != go.logits.len() {
            return Err(format!("logit count mismatch: cpu {} vs gpu {}", co.logits.len(), go.logits.len()));
        }
        for (cl, gl) in co.logits.iter().zip(go.logits.iter()) {
            let ld = (cl - gl).abs();
            if ld > max_logit_diff {
                max_logit_diff = ld;
            }
        }
    }
    if max_logit_diff > tol || max_value_diff > tol {
        return Err(format!(
            "GPU parity failed (tol {tol}): max_logit_diff={max_logit_diff:.6}, max_value_diff={max_value_diff:.6}"
        ));
    }
    Ok(ParityReport { cases: obs.len(), max_logit_diff, max_value_diff })
}
