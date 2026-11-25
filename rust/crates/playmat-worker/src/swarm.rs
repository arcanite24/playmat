//! `playmat-worker run`: the worker's side of the coordinator's API (`playmat_swarm_proto::api`). It leases a unit,
//! fetches and checks its networks, plays it (heartbeating while it does), uploads every file to its presigned URL,
//! and completes the unit with the manifest. With `--when idle` it plays only while the machine is idle, and drops a
//! unit (`/fail`, so it goes to another worker) when the machine stops being idle.

use crate::{idle, read_json, run_unit, sha_file, STRICT};
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{self as proto, ResultManifest};
use reqwest::blocking::Client;
use reqwest::StatusCode;
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

pub fn run(o: Opts) -> Result<(), String> {
    idle::lower_priority();
    let api = Api {
        http: Client::builder().timeout(Duration::from_secs(300)).build().map_err(|e| e.to_string())?,
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
    };
    let reply: HelloReply = api.post("/hello", &hello)?.json().map_err(|e| e.to_string())?;
    println!("WORKER {} (host {}) at {}, class {}, {} jobs", o.name, reply.host, api.base, hello.class, o.jobs);
    let nets_dir = format!("{}/nets", o.work);
    std::fs::create_dir_all(&nets_dir).map_err(|e| format!("{nets_dir}: {e}"))?;
    let mut done = 0;
    loop {
        if o.idle {
            if let Some(why) = idle::busy(false) {
                println!("WAIT {why}");
                std::thread::sleep(Duration::from_secs(reply.poll_secs));
                continue;
            }
        }
        let res = match api.post("/lease", &LeaseRequest { host: reply.host }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("LEASE {e}");
                std::thread::sleep(Duration::from_secs(reply.poll_secs));
                continue;
            }
        };
        if res.status() == StatusCode::NO_CONTENT {
            std::thread::sleep(Duration::from_secs(reply.poll_secs));
            continue;
        }
        let lease: Lease = res.json().map_err(|e| e.to_string())?;
        let id = lease.unit.id.clone();
        match play(&api, &o, &nets_dir, lease) {
            Ok(games) => println!("DONE {id}: {games} games"),
            Err(e) => {
                eprintln!("FAIL {id}: {e}");
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

fn play(api: &Api, o: &Opts, nets_dir: &str, lease: Lease) -> Result<usize, String> {
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

    // heartbeats, and the idle check while it plays: either stops the unit between games
    let stop = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let why: Arc<std::sync::Mutex<Option<String>>> = Arc::default();
    let watcher = {
        let (stop, finished, why) = (stop.clone(), finished.clone(), why.clone());
        let (http, base, token, id, idle_rules) =
            (api.http.clone(), api.base.clone(), api.token.clone(), id.clone(), o.idle);
        let beat = Duration::from_secs((lease.lease_secs / 3).clamp(15, 300));
        std::thread::spawn(move || {
            let (mut beat_at, mut check_at) = (std::time::Instant::now(), std::time::Instant::now());
            while !finished.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_secs(1));
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
    let played = run_unit(&unit_path, &out, nets_dir, &o.repo, o.jobs, &stop);
    finished.store(true, Ordering::Relaxed);
    let _ = watcher.join();
    if let Some(w) = why.lock().unwrap().take() {
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
