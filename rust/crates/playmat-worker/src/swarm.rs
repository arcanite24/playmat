//! `playmat-worker run`: the worker's side of the coordinator's API (`playmat_swarm_proto::api`). It leases a unit,
//! fetches and checks its networks, plays it (heartbeating while it does), uploads every file to its presigned URL,
//! and completes the unit with the manifest. With `--when idle` it plays only while the machine is idle, and drops a
//! unit (`/fail`, so it goes to another worker) when the machine stops being idle.

use crate::{gpu, idle, read_json, run_unit, run_unit_with_infer, sha_file, verify_detailed, STRICT};
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{self as proto, GameSummary, ResultManifest, UnitKind, VerifyMode};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct Opts {
    pub coordinator: String,
    pub token: String,
    pub name: String,
    pub jobs: usize,
    pub work: String,
    pub repo: String,
    /// play only while the machine is idle
    pub idle: bool,
    /// stop after this many units (0 = never)
    pub units: usize,
}

struct Api {
    http: Client,
    base: String,
    token: String,
}

impl Api {
    fn post<B: serde::Serialize>(&self, path: &str, body: &B) -> Result<reqwest::blocking::Response, String> {
        let res = self
            .http
            .post(format!("{}/api/v1{path}", self.base))
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .map_err(|e| format!("{path}: {e}"))?;
        check(path, res)
    }
}

fn check(what: &str, res: reqwest::blocking::Response) -> Result<reqwest::blocking::Response, String> {
    if res.status().is_success() {
        return Ok(res);
    }
    let status = res.status();
    Err(format!("{what}: {status} {}", res.text().unwrap_or_default().chars().take(300).collect::<String>()))
}

#[allow(dead_code)]
pub fn run(o: Opts) -> Result<(), String> {
    run_with_options(o, None, None, "python3".into())
}

pub fn run_with_options(
    o: Opts,
    control: Option<Arc<crate::ui::Control>>,
    gpu_label: Option<String>,
    infer_py: String,
) -> Result<(), String> {
    idle::lower_priority();
    let log = |msg: String| {
        println!("{msg}");
        if let Some(c) = &control {
            c.log(msg);
        }
    };
    let api = Api {
        http: Client::builder()
            .timeout(Duration::from_secs(300))
            .user_agent(concat!("playmat-worker/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?,
        base: o.coordinator.trim_end_matches('/').to_string(),
        token: o.token.clone(),
    };
    let hello = Hello {
        name: o.name.clone(),
        build: option_env!("PLAYMAT_BUILD").unwrap_or("dev").into(),
        class: proto::determinism_class(STRICT),
        strict: STRICT,
        threads: o.jobs as u32,
        os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        gpu: gpu_label.clone(),
    };
    // the coordinator may be down or restarting: keep asking, up to every five minutes (a refused token still stops)
    let mut wait = 5;
    let reply: HelloReply = loop {
        match api.post("/hello", &hello) {
            Ok(r) => break r.json().map_err(|e| e.to_string())?,
            // "/hello: 401 Unauthorized ..." (from `check`): the coordinator answered and said no
            Err(e) if e.split_once(": ").is_some_and(|(_, r)| r.starts_with('4')) => return Err(e),
            Err(e) => {
                eprintln!("HELLO {e}, again in {wait} s");
                if let Some(c) = &control {
                    c.set_status("reconnecting", None);
                    c.log(format!("HELLO {e}, retry in {wait}s"));
                }
                std::thread::sleep(Duration::from_secs(wait));
                wait = (wait * 2).min(300);
            }
        }
    };
    log(format!("WORKER {} (host {}) at {}, class {}, {} jobs", o.name, reply.host, api.base, hello.class, o.jobs));
    let nets_dir = format!("{}/nets", o.work);
    std::fs::create_dir_all(&nets_dir).map_err(|e| format!("{nets_dir}: {e}"))?;
    let mut done = 0;
    let mut waiting = false;
    loop {
        if let Some(c) = &control {
            if c.paused.load(Ordering::Relaxed) {
                c.set_status("paused", None);
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        }
        let idle_only = control.as_ref().map_or(o.idle, |c| c.idle_only.load(Ordering::Relaxed));
        if idle_only {
            if let Some(why) = idle::busy(false) {
                if let Some(c) = &control {
                    c.set_status("waiting (machine busy)", None);
                }
                log(format!("WAIT {why}"));
                std::thread::sleep(Duration::from_secs(reply.poll_secs));
                continue;
            }
        }
        let res = match api.post("/lease", &LeaseRequest { host: reply.host }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("LEASE {e}");
                if let Some(c) = &control {
                    c.log(format!("LEASE {e}"));
                }
                std::thread::sleep(Duration::from_secs(reply.poll_secs));
                continue;
            }
        };
        if res.status() == StatusCode::NO_CONTENT {
            if let Some(c) = &control {
                c.set_status("waiting for work", None);
            }
            if !waiting {
                log(format!(
                    "WAIT no unit for this host ({}, build {}): asking every {} s",
                    hello.class, hello.build, reply.poll_secs
                ));
                waiting = true;
            }
            std::thread::sleep(Duration::from_secs(reply.poll_secs));
            continue;
        }
        waiting = false;
        let lease: Lease = res.json().map_err(|e| e.to_string())?;
        let id = lease.unit.id.clone();
        if let Some(c) = &control {
            c.set_status("working", Some(id.clone()));
            c.log(format!("LEASE {id} ({} games)", lease.unit.games));
        }
        let active_jobs = control.as_ref().map_or(o.jobs, |c| c.jobs.load(Ordering::Relaxed));
        let active_o = Opts {
            coordinator: o.coordinator.clone(),
            token: o.token.clone(),
            name: o.name.clone(),
            jobs: active_jobs,
            work: o.work.clone(),
            repo: o.repo.clone(),
            idle: idle_only,
            units: o.units,
        };
        match play(&api, &active_o, &nets_dir, lease, control.as_ref(), &infer_py) {
            Ok(games) => {
                log(format!("DONE {id}: {games} games"));
                if let Some(c) = &control {
                    c.record_done(games);
                }
            }
            // stopped for the host's sake (not idle, paused, or the lease moved on): hand it back without counting a failure
            Err(e) if e.starts_with("stopped:") => {
                log(format!("RELEASE {id}: {e}"));
                if let Some(c) = &control {
                    c.set_status("paused", None);
                }
                let _ = api.post(&format!("/units/{id}/release"), &serde_json::json!({}));
            }
            Err(e) => {
                eprintln!("FAIL {id}: {e}");
                if let Some(c) = &control {
                    c.log(format!("FAIL {id}: {e}"));
                }
                let _ = api.post(&format!("/units/{id}/fail"), &Fail { reason: e });
            }
        }
        let _ = std::fs::remove_dir_all(format!("{}/units/{id}", o.work));
        done += 1;
        if o.units > 0 && done >= o.units {
            return Ok(());
        }
    }
}

/// Fetches a network unless the cache has it, and checks its hash either way
fn fetch_net(api: &Api, nets_dir: &str, n: &proto::NetRef) -> Result<(), String> {
    let path = format!("{nets_dir}/{}.pmnet", n.sha256);
    if sha_file(&path).ok().as_deref() == Some(n.sha256.as_str()) {
        return Ok(());
    }
    // the coordinator redirects to the store; reqwest drops the bearer token on a redirect to another host
    let res = api.http.get(&n.url).bearer_auth(&api.token).send().map_err(|e| format!("{}: {e}", n.url))?;
    let bytes = check(&n.url, res)?.bytes().map_err(|e| e.to_string())?;
    let sha = proto::sha256_hex(&bytes);
    if sha != n.sha256 {
        return Err(format!("network {}: the download is {sha}", n.sha256));
    }
    let tmp = format!("{path}.part");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("{tmp}: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{path}: {e}"))
}

fn play(
    api: &Api,
    o: &Opts,
    nets_dir: &str,
    lease: Lease,
    control: Option<&Arc<crate::ui::Control>>,
    infer_py: &str,
) -> Result<usize, String> {
    if lease.unit.kind == UnitKind::Verify {
        return play_verify(api, o, nets_dir, lease);
    }
    let id = lease.unit.id.clone();
    proto::check_unit(&lease.unit)?;
    for n in &lease.unit.nets {
        fetch_net(api, nets_dir, n)?;
    }
    let dir = format!("{}/units/{id}", o.work);
    let out = format!("{dir}/out");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{dir}: {e}"))?;
    let unit_path = format!("{dir}/unit.json");
    std::fs::write(&unit_path, serde_json::to_string_pretty(&lease.unit).unwrap()).map_err(|e| e.to_string())?;
    println!("PLAY {id}: {} games from deal {}", lease.unit.games, lease.unit.seed);

    let mut sidecars = Vec::new();
    let mut infer_urls = BTreeMap::new();
    if lease.unit.kind == UnitKind::SelfplayGpu {
        for n in &lease.unit.nets {
            if infer_urls.contains_key(&n.sha256) {
                continue;
            }
            let path = format!("{nets_dir}/{}.pmnet", n.sha256);
            let sc = gpu::InferSidecar::spawn(infer_py, &o.repo, &path, "auto", true)?;
            println!("GPU SIDECAR {} -> {}", &n.sha256[..12.min(n.sha256.len())], sc.url);
            infer_urls.insert(n.sha256.clone(), sc.url.clone());
            sidecars.push(sc);
        }
    }

    let watcher = spawn_watcher(api, o.idle, &id, lease.lease_secs, control.cloned());
    let played = run_unit_with_infer(&unit_path, &out, nets_dir, &o.repo, o.jobs, &watcher.stop, &infer_urls);
    drop(sidecars);
    if let Some(w) = watcher.finish() {
        return Err(format!("stopped: {w}"));
    }
    played?;

    let manifest: ResultManifest = read_json(&format!("{out}/manifest.json"))?;
    let mut files = Vec::new();
    for (path, sha) in &manifest.files {
        let size = std::fs::metadata(format!("{out}/{path}")).map_err(|e| format!("{path}: {e}"))?.len();
        files.push(FileInfo { path: path.clone(), size, sha256: sha.clone() });
    }
    let urls: Urls =
        api.post(&format!("/units/{id}/uploads"), &UploadRequest { files })?.json().map_err(|e| e.to_string())?;
    for (path, url) in &urls.urls {
        let body = std::fs::read(format!("{out}/{path}")).map_err(|e| format!("{path}: {e}"))?;
        let mut tries = 0;
        loop {
            let res = api.http.put(url).header("content-length", body.len()).body(body.clone()).send();
            match res.map_err(|e| e.to_string()).and_then(|r| check(path, r)) {
                Ok(_) => break,
                Err(e) if tries < 3 => {
                    tries += 1;
                    eprintln!("UPLOAD {path}: {e}, again");
                    std::thread::sleep(Duration::from_secs(5 * tries));
                }
                Err(e) => return Err(format!("upload {path}: {e}")),
            }
        }
    }
    let games = manifest.games.len();
    api.post(&format!("/units/{id}/complete"), &Complete { manifest })?;
    Ok(games)
}

struct Watcher {
    stop: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    why: Arc<std::sync::Mutex<Option<String>>>,
    handle: std::thread::JoinHandle<()>,
}

impl Watcher {
    fn finish(self) -> Option<String> {
        self.finished.store(true, Ordering::Relaxed);
        let _ = self.handle.join();
        self.why.lock().unwrap().take()
    }
}

fn spawn_watcher(
    api: &Api,
    idle_rules: bool,
    id: &str,
    lease_secs: u64,
    control: Option<Arc<crate::ui::Control>>,
) -> Watcher {
    let stop = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let why: Arc<std::sync::Mutex<Option<String>>> = Arc::default();
    let handle = {
        let (stop, finished, why) = (stop.clone(), finished.clone(), why.clone());
        let (http, base, token, id) = (api.http.clone(), api.base.clone(), api.token.clone(), id.to_string());
        let beat = Duration::from_secs((lease_secs / 3).clamp(15, 300));
        std::thread::spawn(move || {
            let (mut beat_at, mut check_at) = (std::time::Instant::now(), std::time::Instant::now());
            while !finished.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
                if let Some(c) = &control {
                    if c.paused.load(Ordering::Relaxed) {
                        *why.lock().unwrap() = Some("paused from local UI".into());
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                }
                if idle_rules && check_at.elapsed() >= Duration::from_secs(15) {
                    check_at = std::time::Instant::now();
                    if let Some(w) = idle::busy(true) {
                        *why.lock().unwrap() = Some(w);
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                }
                if beat_at.elapsed() >= beat {
                    beat_at = std::time::Instant::now();
                    let res = http.post(format!("{base}/api/v1/units/{id}/heartbeat")).bearer_auth(&token).send();
                    if res.is_ok_and(|r| r.status() == StatusCode::CONFLICT) {
                        *why.lock().unwrap() = Some("the lease went to another host".into());
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            }
        })
    };
    Watcher { stop, finished, why, handle }
}

/// Executes a `UnitKind::Verify` unit (either `Replay` or `Rerun`) and posts its `VerifyReport`.
fn play_verify(api: &Api, o: &Opts, nets_dir: &str, lease: Lease) -> Result<usize, String> {
    let id = lease.unit.id.clone();
    proto::check_unit(&lease.unit)?;
    let spec = lease.unit.verify.clone().ok_or("verify unit missing spec")?;
    let dir = format!("{}/units/{id}", o.work);
    let orig_dir = format!("{dir}/orig");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&orig_dir).map_err(|e| format!("{orig_dir}: {e}"))?;

    println!("VERIFY {id}: {:?} of {} ({} games)", spec.mode, spec.target_unit, lease.unit.games);
    let started = std::time::Instant::now();

    // Download every uploaded file and verify its sha256 against the claimed manifest
    let mut file_mismatches: Vec<String> = Vec::new();
    for (path, expected_sha) in &spec.manifest.files {
        if path.contains('/') || path.contains('\\') || path.contains("..") {
            file_mismatches.push(path.clone());
            continue;
        }
        let Some(url) = spec.files.get(path) else {
            file_mismatches.push(path.clone());
            continue;
        };
        let res = api.http.get(url).send().map_err(|e| format!("download {path}: {e}"))?;
        let bytes = check(path, res)?.bytes().map_err(|e| e.to_string())?;
        let got_sha = proto::sha256_hex(&bytes);
        if got_sha != *expected_sha {
            file_mismatches.push(path.clone());
        }
        std::fs::write(format!("{orig_dir}/{path}"), &bytes).map_err(|e| format!("{path}: {e}"))?;
    }

    let mut target_unit = lease.unit.clone();
    target_unit.id = spec.target_unit.clone();
    target_unit.kind = spec.target_kind;
    target_unit.verify = None;

    let report = match spec.mode {
        VerifyMode::Replay => {
            std::fs::write(format!("{orig_dir}/unit.json"), serde_json::to_string_pretty(&target_unit).unwrap())
                .map_err(|e| e.to_string())?;
            std::fs::write(format!("{orig_dir}/manifest.json"), serde_json::to_string_pretty(&spec.manifest).unwrap())
                .map_err(|e| e.to_string())?;
            let out = verify_detailed(&orig_dir, &o.repo)?;
            for f in out.file_mismatches {
                if !file_mismatches.contains(&f) {
                    file_mismatches.push(f);
                }
            }
            let passed = out.passed && file_mismatches.is_empty();
            let reason = if passed || !out.passed {
                out.reason
            } else {
                format!("uploaded file hash mismatch: {file_mismatches:?}")
            };
            VerifyReport {
                mode: spec.mode,
                target_unit: spec.target_unit.clone(),
                passed,
                reason,
                mismatches: out.mismatches,
                file_mismatches,
                elapsed_ms: started.elapsed().as_millis() as u64,
            }
        }
        VerifyMode::Rerun => {
            for n in &target_unit.nets {
                fetch_net(api, nets_dir, n)?;
            }
            let target_unit_path = format!("{dir}/target_unit.json");
            let rerun_dir = format!("{dir}/rerun");
            std::fs::write(&target_unit_path, serde_json::to_string_pretty(&target_unit).unwrap())
                .map_err(|e| e.to_string())?;

            let watcher = spawn_watcher(api, o.idle, &id, lease.lease_secs, None);
            let played = run_unit(&target_unit_path, &rerun_dir, nets_dir, &o.repo, o.jobs, &watcher.stop);
            if let Some(w) = watcher.finish() {
                return Err(format!("stopped: {w}"));
            }
            played?;

            let rerun: ResultManifest = read_json(&format!("{rerun_dir}/manifest.json"))?;
            compare_rerun(
                &spec.manifest,
                &rerun,
                file_mismatches,
                spec.target_unit.clone(),
                started.elapsed().as_millis() as u64,
            )
        }
    };

    let games = lease.unit.games as usize;
    api.post(&format!("/units/{id}/verify-complete"), &report)?;
    println!(
        "VERIFIED {id} ({:?} of {}): {} ({})",
        report.mode,
        report.target_unit,
        if report.passed { "PASS" } else { "MISMATCH" },
        report.reason
    );
    Ok(games)
}

fn compare_rerun(
    claimed: &ResultManifest,
    rerun: &ResultManifest,
    mut file_mismatches: Vec<String>,
    target_unit: String,
    elapsed_ms: u64,
) -> VerifyReport {
    let mut problems: Vec<String> = Vec::new();
    let mut mismatches: Vec<u64> = Vec::new();

    if claimed.failed != rerun.failed {
        problems.push(format!("failed deals differ: claimed {:?}, rerun {:?}", claimed.failed, rerun.failed));
        mismatches.extend(&claimed.failed);
        mismatches.extend(&rerun.failed);
    }

    let c_map: BTreeMap<u64, &GameSummary> = claimed.games.iter().map(|g| (g.deal, g)).collect();
    let r_map: BTreeMap<u64, &GameSummary> = rerun.games.iter().map(|g| (g.deal, g)).collect();
    for (&deal, rg) in &r_map {
        match c_map.get(&deal) {
            None => {
                problems.push(format!("deal {deal}: missing from claimed manifest"));
                mismatches.push(deal);
            }
            Some(cg) => {
                if cg.winner != rg.winner
                    || cg.learner != rg.learner
                    || cg.opponent != rg.opponent
                    || cg.turns != rg.turns
                    || cg.answers != rg.answers
                    || cg.recorded != rg.recorded
                    || cg.log_sha256 != rg.log_sha256
                    || cg.steps_sha256 != rg.steps_sha256
                {
                    problems.push(format!(
                        "deal {deal}: mismatch (log {}, steps {})",
                        if cg.log_sha256 == rg.log_sha256 { "same" } else { "DIFF" },
                        if cg.steps_sha256 == rg.steps_sha256 { "same" } else { "DIFF" }
                    ));
                    mismatches.push(deal);
                }
            }
        }
    }
    for &deal in c_map.keys() {
        if !r_map.contains_key(&deal) {
            problems.push(format!("deal {deal}: extra game in claimed manifest"));
            mismatches.push(deal);
        }
    }

    // Compare every uploaded file's sha256 against the bit-exact rerun shard files
    for (name, r_sha) in &rerun.files {
        match claimed.files.get(name) {
            Some(c_sha) if c_sha == r_sha => {}
            _ => {
                problems.push(format!("file {name}: hash differs from rerun"));
                if !file_mismatches.contains(name) {
                    file_mismatches.push(name.clone());
                }
            }
        }
    }
    for name in claimed.files.keys() {
        if !rerun.files.contains_key(name) {
            problems.push(format!("file {name}: unexpected extra file"));
            if !file_mismatches.contains(name) {
                file_mismatches.push(name.clone());
            }
        }
    }
    if !file_mismatches.is_empty() && problems.is_empty() {
        problems.push(format!("uploaded file hash mismatch: {file_mismatches:?}"));
    }

    mismatches.sort();
    mismatches.dedup();
    file_mismatches.sort();
    file_mismatches.dedup();
    let passed = problems.is_empty() && mismatches.is_empty() && file_mismatches.is_empty();
    let reason = if passed {
        format!("rerun matched {} games and {} files bit-for-bit", rerun.games.len(), rerun.files.len())
    } else {
        problems.iter().take(5).cloned().collect::<Vec<_>>().join("; ")
    };
    VerifyReport { mode: VerifyMode::Rerun, target_unit, passed, reason, mismatches, file_mismatches, elapsed_ms }
}
