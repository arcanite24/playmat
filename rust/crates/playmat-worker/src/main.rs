//! Mana@Home's worker, the local part (S0, docs/swarm/Design.md): no coordinator yet, units come from files.
//!
//! playmat-worker run-unit UNIT.json --out DIR --nets DIR [--repo DIR] [--jobs N]
//!     checks the unit, resolves its `net=@<sha256>` to `NETS/<sha256>.pmnet` (checking each file's hash), plays its
//!     deals and writes the shard, `answers.jsonl` (every game's answer log), `unit.json` and `manifest.json` to DIR
//! playmat-worker verify DIR [--repo DIR]
//!     replays every game of a result from its answer logs alone and checks each against the manifest
//! playmat-worker compare DIR DIR...
//!     whether results of the same unit played every game alike (the determinism audit)
//! playmat-worker class
//!     this machine's determinism class
//! playmat-worker run --coordinator URL --name HOST [--token-file F] [--jobs N] [--work DIR] [--when idle|always]
//!     [--units N]: plays units from a coordinator (`swarm.rs`); the token comes from `--token-file` or MANA_TOKEN.
//!     Every option can also be set as MANA_<NAME> (MANA_COORDINATOR, MANA_NAME, MANA_JOBS, MANA_WHEN, MANA_REPO...)
//!
//! Games run one per thread with `--jobs` threads; a unit's configs can't ask for more threads per game.

#[cfg(test)]
mod e2e_test;
mod gpu;
mod idle;
mod swarm;
mod ui;

use playmat_ai::answerlog::AnswerLog;
use playmat_ai::entrants::{entrant, Entrant};
use playmat_ai::play::{self, Pairing, Setup};
use playmat_ai::selfplay::Shard;
use playmat_swarm_proto::{self as proto, GameSummary, ResultManifest, UnitKind, WorkUnit};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::exit;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

const STRICT: bool = playmat_ai::STRICT;

fn main() {
    // one thread per game, and no policy from the environment: `Config::default` reads both, and its thread count
    // follows the machine's cores (set before any thread starts)
    std::env::set_var("PLAYMAT_AI_THREADS", "1");
    std::env::remove_var("PLAYMAT_AI_POLICY");
    let args: Vec<String> = std::env::args().collect();
    // every option of `run` can also come from the environment (the Docker image): --coordinator is MANA_COORDINATOR
    let opt = |name: &str| {
        args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned().or_else(|| {
            std::env::var(format!("MANA_{}", name.trim_start_matches('-').replace('-', "_").to_uppercase())).ok()
        })
    };
    let repo = opt("--repo").unwrap_or_else(|| env!("CARGO_MANIFEST_DIR").to_string() + "/../../..");
    let res = match args.get(1).map(String::as_str) {
        Some("class") => {
            println!("{}", proto::determinism_class(STRICT));
            Ok(())
        }
        Some("run-unit") => {
            let mut infer_urls = BTreeMap::new();
            if let Some(spec) = opt("--infer") {
                for pair in spec.split(',') {
                    if let Some((sha, url)) = pair.split_once('=') {
                        infer_urls.insert(sha.trim().to_string(), url.trim().to_string());
                    }
                }
            }
            run_unit_with_infer(
                args.get(2).expect("UNIT.json"),
                &opt("--out").expect("--out DIR"),
                &opt("--nets").expect("--nets DIR"),
                &repo,
                opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or(1),
                &AtomicBool::new(false),
                &infer_urls,
            )
        }
        Some("parity") => (|| {
            let pmnet = opt("--pmnet").ok_or("--pmnet FILE")?;
            let cases = opt("--cases").and_then(|v| v.parse().ok()).unwrap_or(16);
            let tol = opt("--tol").and_then(|v| v.parse().ok()).unwrap_or(0.05);
            let (_sidecar, url) = match opt("--infer") {
                Some(u) => (None, u),
                None => {
                    let py = opt("--infer-py").unwrap_or_else(|| "python3".into());
                    let dev = opt("--device").unwrap_or_else(|| "auto".into());
                    let amp = opt("--amp").as_deref() != Some("0");
                    let sc = gpu::InferSidecar::spawn(&py, &repo, &pmnet, &dev, amp)?;
                    let u = sc.url.clone();
                    (Some(sc), u)
                }
            };
            let rep = gpu::check_parity(&pmnet, &url, cases, tol)?;
            println!(
                "PARITY OK ({} cases, tol {}): max_logit_diff={:.6}, max_value_diff={:.6}",
                rep.cases, tol, rep.max_logit_diff, rep.max_value_diff
            );
            Ok(())
        })(),
        Some("verify") => verify(args.get(2).expect("DIR"), &repo),
        Some("run") | Some("ui") => (|| {
            let with_ui = args.get(1).map(String::as_str) == Some("ui");
            let token = match opt("--token-file") {
                Some(f) => std::fs::read_to_string(&f).map_err(|e| format!("{f}: {e}"))?.trim().to_string(),
                None => std::env::var("MANA_TOKEN").map_err(|_| "--token-file F or MANA_TOKEN")?,
            };
            let jobs = opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                // half the cores by default: the machine stays usable
                std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).max(1))
            });
            let coordinator = opt("--coordinator").ok_or("--coordinator URL")?;
            let name = opt("--name").ok_or("--name HOST")?;
            let idle_only = opt("--when").as_deref() != Some("always");
            let gpu_label = match opt("--gpu").as_deref() {
                None | Some("") | Some("0") | Some("off") => None,
                Some("auto") | Some("1") => gpu::detect_gpu(),
                Some(explicit) => Some(explicit.to_string()),
            };
            let infer_py = opt("--infer-py").unwrap_or_else(|| "python3".into());
            let opts = swarm::Opts {
                coordinator: coordinator.clone(),
                token,
                name: name.clone(),
                jobs,
                work: opt("--work").unwrap_or_else(|| "mana-work".into()),
                repo: repo.clone(),
                idle: idle_only,
                units: opt("--units").and_then(|v| v.parse().ok()).unwrap_or(0),
            };
            let ctrl = if with_ui {
                let port = opt("--port").and_then(|v| v.parse().ok()).unwrap_or(17195);
                let c = ui::Control::new(name, coordinator, proto::determinism_class(STRICT), jobs, idle_only);
                let addr = ui::spawn_server(port, c.clone())?;
                println!("UI http://{addr}");
                Some(c)
            } else {
                None
            };
            swarm::run_with_options(opts, ctrl, gpu_label, infer_py)
        })(),
        Some("compare") => compare(&args[2..]),
        _ => Err("usage: playmat-worker run|ui|run-unit|verify|parity|compare|class (see the source's header)".into()),
    };
    if let Err(e) = res {
        eprintln!("playmat-worker: {e}");
        exit(1);
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
}

fn sha_file(path: &str) -> Result<String, String> {
    Ok(proto::sha256_hex(&std::fs::read(path).map_err(|e| format!("{path}: {e}"))?))
}

/// The unit's suite: its folder under `archbench/`, checked against the pinned hash
fn suite(unit: &WorkUnit, repo: &str) -> Result<(Value, Vec<playmat_ai::duel::Deck>), String> {
    let dir = format!("{repo}/archbench/{}", unit.pins.suite);
    let path = format!("{dir}/suite.json");
    let sha = proto::suite_sha256(&std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?);
    if sha != unit.pins.suite_sha256 {
        return Err(format!(
            "suite {}: suite.json is {sha}, the unit pins {}",
            unit.pins.suite, unit.pins.suite_sha256
        ));
    }
    play::load_suite(repo, &dir)
}

/// A config with its `@<sha256>` networks swapped for the checked files
fn resolve(config: &str, nets: &BTreeMap<String, String>) -> Result<Entrant, String> {
    if config == "self" {
        return Err("self".into());
    }
    let mut c = config.to_string();
    if c.starts_with("cfg:") {
        // the last setting of a key wins: whatever the unit says, one thread per game
        c.push_str("+threads=1+par=1");
    }
    for (sha, path) in nets {
        c = c.replace(&format!("@{sha}"), path);
    }
    entrant(&c)
}

/// Mean Shannon entropy of recorded policy targets (with > 1 candidate) in a game, for S2 statistical checks.
fn mean_policy_entropy(steps: &[playmat_ai::selfplay::Step]) -> Option<f32> {
    let mut sum = 0.0f64;
    let mut n = 0u64;
    for s in steps {
        if s.target.len() <= 1 {
            continue;
        }
        let mut h = 0.0f64;
        for &p in &s.target {
            let p = p as f64;
            if p > 1e-12 {
                h -= p * p.ln();
            }
        }
        if h.is_finite() {
            sum += h;
            n += 1;
        }
    }
    (n > 0).then_some((sum / n as f64) as f32)
}

struct PlayedRow {
    summary: GameSummary,
    log: String,
    shard: Option<(Vec<playmat_ai::selfplay::Step>, [f32; 2], Value)>,
}

/// Plays a unit into `out`. `stop` ends it between games (the host stopped being idle): the result is then an error
pub fn run_unit(
    unit_path: &str,
    out: &str,
    nets_dir: &str,
    repo: &str,
    jobs: usize,
    stop: &AtomicBool,
) -> Result<(), String> {
    run_unit_with_infer(unit_path, out, nets_dir, repo, jobs, stop, &BTreeMap::new())
}

pub fn run_unit_with_infer(
    unit_path: &str,
    out: &str,
    nets_dir: &str,
    repo: &str,
    jobs: usize,
    stop: &AtomicBool,
    infer_urls: &BTreeMap<String, String>,
) -> Result<(), String> {
    let unit: WorkUnit = read_json(unit_path)?;
    proto::check_unit(&unit)?;
    if unit.kind != UnitKind::SelfplayGpu && unit.pins.strict != STRICT {
        let build = if unit.pins.strict {
            "cargo build --release -p playmat-worker --features strict"
        } else {
            "a build without --features strict"
        };
        return Err(format!("the unit wants strict={}, this build is strict={STRICT}: run {build}", unit.pins.strict));
    }
    if unit.kind == UnitKind::Verify {
        return Err("Verify units can't be run via run-unit".into());
    }
    let mut nets = BTreeMap::new();
    for n in &unit.nets {
        let path = format!("{nets_dir}/{}.pmnet", n.sha256);
        let sha = sha_file(&path)?;
        if sha != n.sha256 {
            return Err(format!("{path}: the file is {sha}"));
        }
        let target = infer_urls.get(&n.sha256).cloned().unwrap_or(path);
        nets.insert(n.sha256.clone(), target);
    }
    let (suite_json, decks) = suite(&unit, repo)?;
    let learner = resolve(&unit.learner, &nets)?;
    let mut opponents: Vec<Option<Entrant>> = Vec::new();
    for o in &unit.opponents {
        opponents.push(if o.config == "self" { None } else { Some(resolve(&o.config, &nets)?) });
    }
    let weights: Vec<u64> = unit.opponents.iter().map(|o| o.weight).collect();
    let (fast, fast_iters) = unit.fast.as_deref().map_or(Ok((None, None)), play::parse_fast)?;
    let rec = play::Recorder { keep: unit.keep, fast, fast_iters };
    let db = playmat_engine::cards();
    let setup = Setup {
        db: &db,
        decks: &decks,
        turns: suite_json["turn_cap"].as_u64().unwrap_or(30) as u32,
        mulligans: suite_json["mulligans"].as_bool().unwrap_or(false),
    };
    let recording = matches!(unit.kind, UnitKind::Selfplay | UnitKind::SelfplayGpu);

    std::fs::create_dir_all(out).map_err(|e| format!("{out}: {e}"))?;
    std::fs::copy(unit_path, format!("{out}/unit.json")).map_err(|e| format!("{out}/unit.json: {e}"))?;
    let started = std::time::Instant::now();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<PlayedRow>> = Mutex::new(Vec::new());
    let failed: Mutex<Vec<u64>> = Mutex::new(Vec::new());
    let games = unit.games as usize;
    std::thread::scope(|s| {
        for _job in 0..jobs.max(1) {
            let (setup, learner, opponents, weights, next, results, failed, unit) =
                (&setup, &learner, &opponents, &weights, &next, &results, &failed, &unit);
            let stop = &*stop;
            s.spawn(move || loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let g = next.fetch_add(1, Ordering::Relaxed);
                if g >= games {
                    break;
                }
                let deal = unit.seed as usize + g;
                let p = play::pairing(deal, setup.decks.len(), weights);
                let opp = opponents[p.opponent].as_ref().unwrap_or(learner);
                let mut configs = [opp.config.clone(), opp.config.clone()];
                configs[p.me] = learner.config.clone();
                let both = opponents[p.opponent].is_none();
                let record = [recording && (both || p.me == 0), recording && (both || p.me == 1)];
                let run = catch_unwind(AssertUnwindSafe(|| play::play_game(setup, deal, p, configs, record, rec)));
                let Ok(Ok(played)) = run else {
                    failed.lock().unwrap().push(deal as u64);
                    continue;
                };
                let summary = GameSummary {
                    deal: deal as u64,
                    winner: played.winner.map(|w| w as u8),
                    learner: p.me as u8,
                    opponent: p.opponent as u32,
                    turns: played.turns,
                    answers: played.log.entries.len() as u32,
                    recorded: played.steps.len() as u32,
                    policy_entropy: mean_policy_entropy(&played.steps),
                    log_sha256: played.log.hash(),
                    steps_sha256: play::steps_hash(&played.steps),
                };
                let shard = if recording {
                    let result = match played.winner {
                        Some(w) => [(w == 0) as u8 as f32, (w == 1) as u8 as f32],
                        None => [0.5, 0.5],
                    };
                    let [a, b] = p.decks;
                    let info = json!({
                        "deal": deal, "decks": [setup.decks[a].name, setup.decks[b].name], "learner": p.me,
                        "opponent": opp.name, "turns": played.turns, "first": p.first, "unit": unit.id,
                    });
                    Some((played.steps, result, info))
                } else {
                    None
                };
                results.lock().unwrap().push(PlayedRow { summary, log: played.log.encode(), shard });
            });
        }
    });
    if stop.load(Ordering::Relaxed) {
        return Err("stopped: the host isn't idle".into());
    }
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| r.summary.deal);
    // write one shard `w.*` in sorted deal order so shard file hashes are bit-identical at any --jobs count
    if recording {
        let mut shard = Shard::create(out, "w").map_err(|e| format!("shard: {e}"))?;
        for r in &mut results {
            if let Some((steps, result, info)) = r.shard.take() {
                shard
                    .game(r.summary.deal as usize, unit.round as i32, &steps, result, info)
                    .map_err(|e| format!("shard: {e}"))?;
            }
        }
        shard.flush().map_err(|e| format!("shard flush: {e}"))?;
    }
    let mut failed = failed.into_inner().unwrap();
    failed.sort();
    let answers_path = format!("{out}/answers.jsonl");
    {
        let mut f = std::io::BufWriter::new(std::fs::File::create(&answers_path).map_err(|e| e.to_string())?);
        for r in &results {
            writeln!(f, "{}", json!({"deal": r.summary.deal, "log_sha256": r.summary.log_sha256, "log": r.log}))
                .map_err(|e| e.to_string())?;
        }
    }
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(out).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        // unit.json stays home: whoever checks a result replays it against the unit the coordinator leased
        if name != "manifest.json" && name != "unit.json" && entry.path().is_file() {
            files.insert(name.clone(), sha_file(&format!("{out}/{name}"))?);
        }
    }
    let manifest = ResultManifest {
        protocol: proto::PROTOCOL,
        unit: unit.id.clone(),
        worker: std::env::var("PLAYMAT_WORKER").unwrap_or_else(|_| "local".into()),
        build: option_env!("PLAYMAT_BUILD").unwrap_or("dev").into(),
        class: proto::determinism_class(STRICT),
        nets: unit.nets.iter().map(|n| n.sha256.clone()).collect(),
        games: results.into_iter().map(|r| r.summary).collect(),
        failed,
        files,
        elapsed_ms: started.elapsed().as_millis() as u64,
    };
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(format!("{out}/manifest.json"), text).map_err(|e| e.to_string())?;
    let secs = started.elapsed().as_secs_f64();
    println!(
        "RUN {}: {} games ({} failed) in {secs:.1} s, {:.2} games/s, class {}",
        unit.id,
        manifest.games.len(),
        manifest.failed.len(),
        manifest.games.len() as f64 / secs,
        manifest.class
    );
    Ok(())
}

pub struct VerifyOutcome {
    pub passed: bool,
    pub reason: String,
    pub mismatches: Vec<u64>,
    pub file_mismatches: Vec<String>,
    pub elapsed_ms: u64,
}

pub fn verify_detailed(dir: &str, repo: &str) -> Result<VerifyOutcome, String> {
    let unit: WorkUnit = read_json(&format!("{dir}/unit.json"))?;
    proto::check_unit(&unit)?;
    let manifest: ResultManifest = read_json(&format!("{dir}/manifest.json"))?;
    let mut problems: Vec<String> = Vec::new();
    let mut mismatches: Vec<u64> = Vec::new();
    let mut file_mismatches: Vec<String> = Vec::new();
    for (name, sha) in &manifest.files {
        if name.contains('/') || name.contains("..") {
            problems.push(format!("{name}: not a plain file name"));
            file_mismatches.push(name.clone());
        } else {
            match sha_file(&format!("{dir}/{name}")) {
                Ok(got) if got == *sha => {}
                Ok(_) => {
                    problems.push(format!("{name}: its hash differs from the manifest"));
                    file_mismatches.push(name.clone());
                }
                Err(e) => {
                    problems.push(e);
                    file_mismatches.push(name.clone());
                }
            }
        }
    }
    if unit.kind == UnitKind::Selfplay {
        let meta_path = format!("{dir}/w.meta.i32");
        if let Ok(meta) = std::fs::metadata(&meta_path) {
            let recorded: u64 = manifest.games.iter().map(|g| g.recorded as u64).sum();
            let want_bytes = recorded * playmat_ai::selfplay::META as u64 * 4;
            if meta.len() != want_bytes {
                problems
                    .push(format!("w.meta.i32: {} bytes, expected {want_bytes} for {recorded} decisions", meta.len()));
                file_mismatches.push("w.meta.i32".into());
            }
        }
    }
    let (suite_json, decks) = suite(&unit, repo)?;
    let db = playmat_engine::cards();
    let setup = Setup {
        db: &db,
        decks: &decks,
        turns: suite_json["turn_cap"].as_u64().unwrap_or(30) as u32,
        mulligans: suite_json["mulligans"].as_bool().unwrap_or(false),
    };
    let weights: Vec<u64> = unit.opponents.iter().map(|o| o.weight).collect();
    // every deal of the unit is a game or a failure, once
    let mut deals: Vec<u64> = manifest.games.iter().map(|g| g.deal).chain(manifest.failed.iter().copied()).collect();
    deals.sort();
    let want: Vec<u64> = (unit.seed..unit.seed + unit.games as u64).collect();
    if deals != want {
        problems.push(format!("the result covers {} deals, the unit has {}", deals.len(), want.len()));
    }
    let claimed: BTreeMap<u64, &GameSummary> = manifest.games.iter().map(|g| (g.deal, g)).collect();
    let file = std::fs::File::open(format!("{dir}/answers.jsonl")).map_err(|e| format!("answers.jsonl: {e}"))?;
    let started = std::time::Instant::now();
    let mut replayed = 0;
    let mut answers = 0usize;
    for line in std::io::BufReader::new(file).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&line).map_err(|e| format!("answers.jsonl: {e}"))?;
        let deal = v["deal"].as_u64().ok_or("answers.jsonl: no deal")?;
        let Some(claim) = claimed.get(&deal) else {
            problems.push(format!("deal {deal}: an answer log for a game the manifest doesn't list"));
            mismatches.push(deal);
            continue;
        };
        let log = match AnswerLog::decode(v["log"].as_str().unwrap_or("")) {
            Ok(l) => l,
            Err(e) => {
                problems.push(format!("deal {deal}: {e}"));
                mismatches.push(deal);
                continue;
            }
        };
        if log.hash() != claim.log_sha256 {
            problems.push(format!("deal {deal}: the log's hash differs from the manifest"));
            mismatches.push(deal);
            continue;
        }
        let p: Pairing = play::pairing(deal as usize, decks.len(), &weights);
        if p.me as u8 != claim.learner || p.opponent as u32 != claim.opponent {
            problems.push(format!("deal {deal}: the learner's seat or opponent differs from what the deal decides"));
            mismatches.push(deal);
        }
        answers += log.entries.len();
        match catch_unwind(AssertUnwindSafe(|| play::replay_game(&setup, deal as usize, p, &log))) {
            Ok(Ok(r)) => {
                if r.winner.map(|w| w as u8) != claim.winner || r.turns != claim.turns {
                    problems.push(format!(
                        "deal {deal}: replayed {:?} in {} turns, the manifest says {:?} in {}",
                        r.winner, r.turns, claim.winner, claim.turns
                    ));
                    mismatches.push(deal);
                }
            }
            Ok(Err(e)) => {
                problems.push(e);
                mismatches.push(deal);
            }
            Err(_) => {
                problems.push(format!("deal {deal}: the replay panicked"));
                mismatches.push(deal);
            }
        }
        replayed += 1;
    }
    if replayed != manifest.games.len() {
        problems.push(format!("{} games listed, {replayed} answer logs", manifest.games.len()));
    }
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let summary = format!(
        "VERIFY {}: {replayed} games, {answers} answers replayed in {:.2} s; {} problems",
        manifest.unit,
        started.elapsed().as_secs_f64(),
        problems.len()
    );
    println!("{summary}");
    for p in problems.iter().take(20) {
        println!("  {p}");
    }
    let passed = problems.is_empty();
    let reason = if passed { summary } else { problems.iter().take(5).cloned().collect::<Vec<_>>().join("; ") };
    mismatches.sort();
    mismatches.dedup();
    file_mismatches.sort();
    file_mismatches.dedup();
    Ok(VerifyOutcome { passed, reason, mismatches, file_mismatches, elapsed_ms })
}

fn verify(dir: &str, repo: &str) -> Result<(), String> {
    let out = verify_detailed(dir, repo)?;
    if out.passed {
        Ok(())
    } else {
        Err(out.reason)
    }
}

fn compare(dirs: &[String]) -> Result<(), String> {
    if dirs.len() < 2 {
        return Err("compare needs two results or more".into());
    }
    let manifests: Vec<ResultManifest> =
        dirs.iter().map(|d| read_json(&format!("{d}/manifest.json"))).collect::<Result<_, _>>()?;
    let first: BTreeMap<u64, &GameSummary> = manifests[0].games.iter().map(|g| (g.deal, g)).collect();
    for (d, m) in dirs.iter().zip(&manifests).skip(1) {
        let (mut same, mut data) = (0, 0);
        let mut differ = Vec::new();
        for g in &m.games {
            match first.get(&g.deal) {
                Some(f) if f.log_sha256 == g.log_sha256 => {
                    same += 1;
                    if f.steps_sha256 != g.steps_sha256 {
                        data += 1;
                    }
                }
                Some(_) => differ.push(g.deal),
                None => {}
            }
        }
        println!(
            "COMPARE {} ({}) vs {d} ({}): {same} alike, {} differ{}{}",
            dirs[0],
            manifests[0].class,
            m.class,
            differ.len(),
            if data > 0 { format!(", {data} alike in answers but not in training data") } else { String::new() },
            if differ.is_empty() { String::new() } else { format!(", first {:?}", &differ[..differ.len().min(10)]) }
        );
    }
    Ok(())
}
