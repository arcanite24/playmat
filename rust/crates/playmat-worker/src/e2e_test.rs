//! A round through a real coordinator (in-process, with its local store): the trainer publishes a two-game unit, the
//! worker's run loop leases, plays, uploads and completes it, and the trainer downloads the result, replays it
//! against the unit the coordinator leased, and accepts it.

use crate::{swarm, verify, STRICT};
use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{Opponent, Pins, UnitKind, WorkUnit, PROTOCOL};
use reqwest::blocking::Client;

#[test]
fn a_round_goes_through_the_coordinator() {
    std::env::set_var("PLAYMAT_AI_THREADS", "1");
    let repo = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("mana-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let listener = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = rt.block_on(Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base)).unwrap();
    let worker = rt.block_on(s.new_token(Role::Worker, "alice")).unwrap().token;
    let trainer = rt.block_on(s.new_token(Role::Trainer, "trainer")).unwrap().token;
    rt.spawn(playmat_swarm::serve(listener, s));

    let unit = WorkUnit {
        protocol: PROTOCOL,
        id: "t-r1-u000".into(),
        project: "test".into(),
        run: "e2e".into(),
        round: 1,
        kind: UnitKind::Selfplay,
        pins: Pins {
            builds: vec![],
            obs: 3,
            suite: "constructed-2".into(),
            suite_sha256: playmat_swarm_proto::suite_sha256(
                &std::fs::read(format!("{repo}/archbench/constructed-2/suite.json")).unwrap(),
            ),
            strict: STRICT,
        },
        nets: vec![],
        learner: "cfg:policy=ismcts+net=builtin+hide=1+iters=4+samples=1".into(),
        opponents: vec![Opponent { config: "host@s6".into(), weight: 1 }],
        seed: 700_000,
        games: 2,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 1.0,
        verify: None,
    };
    let http = Client::new();
    let api = |path: &str| format!("{base}/api/v1{path}");
    let round =
        NewRound { project: "test".into(), run: "e2e".into(), round: 1, priority: 0, units: vec![unit.clone()] };
    let res = http.post(api("/rounds")).bearer_auth(&trainer).json(&round).send().unwrap();
    assert!(res.status().is_success(), "{}", res.text().unwrap());
    let round: RoundStatus = res.json().unwrap();
    // a worker token can't publish rounds
    let res = http.post(api("/rounds")).bearer_auth(&worker).json(&NewRound { round: 2, ..Default::default() }).send();
    assert_eq!(res.unwrap().status(), 403);

    swarm::run(swarm::Opts {
        coordinator: base.clone(),
        token: worker.clone(),
        name: "alice-laptop".into(),
        jobs: 2,
        work: tmp.join("work").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 1,
    })
    .unwrap();

    let results: Vec<UnitResult> =
        http.get(api(&format!("/rounds/{}/results", round.id))).bearer_auth(&trainer).send().unwrap().json().unwrap();
    assert_eq!(results.len(), 1);
    let r = &results[0];
    assert_eq!((r.account.as_str(), r.host.as_str(), r.manifest.games.len()), ("alice", "alice-laptop", 2));
    assert!(!r.files.contains_key("unit.json"));
    // the two threads' shards come as one: a unit's upload doesn't grow with its threads
    assert!(r.files.contains_key("w.meta.i32") && !r.files.keys().any(|f| f.starts_with("w0")), "{:?}", r.files.keys());
    assert_eq!(r.files.len(), playmat_ai::selfplay::COLUMNS.len() + 1);
    let dir = tmp.join("check");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("unit.json"), serde_json::to_string(&r.work).unwrap()).unwrap();
    std::fs::write(dir.join("manifest.json"), serde_json::to_string(&r.manifest).unwrap()).unwrap();
    for (path, url) in &r.files {
        std::fs::write(dir.join(path), http.get(url).send().unwrap().bytes().unwrap()).unwrap();
    }
    verify(dir.to_str().unwrap(), &repo).unwrap();
    let recorded: u64 = r.manifest.games.iter().map(|g| g.recorded as u64).sum();
    let meta = std::fs::metadata(dir.join("w.meta.i32")).unwrap().len();
    assert_eq!(meta, recorded * playmat_ai::selfplay::META as u64 * 4);

    let v = Verdict { accept: true, reason: "replayed".into() };
    let res = http.post(api(&format!("/units/{}/verdict", r.unit))).bearer_auth(&trainer).json(&v).send().unwrap();
    assert!(res.status().is_success());
    let st: Status = http.get(api("/status")).send().unwrap().json().unwrap();
    assert_eq!((st.games_accepted, st.hosts, st.units.get("accepted").copied()), (2, 1, Some(1)));
    let _ = std::fs::remove_dir_all(&tmp);
}

fn test_unit(repo: &str, id: &str, seed: u64, games: u32, learner: &str) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "test".into(),
        run: "s2-e2e".into(),
        round: 1,
        kind: UnitKind::Selfplay,
        pins: Pins {
            builds: vec![],
            obs: 3,
            suite: "constructed-2".into(),
            suite_sha256: playmat_swarm_proto::suite_sha256(
                &std::fs::read(format!("{repo}/archbench/constructed-2/suite.json")).unwrap(),
            ),
            strict: STRICT,
        },
        nets: vec![],
        learner: learner.into(),
        opponents: vec![Opponent { config: "host@s6".into(), weight: 1 }],
        seed,
        games,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 10.0,
        verify: None,
    }
}

#[test]
fn s2_autonomous_rerun_verification_accepts_and_credits() {
    std::env::set_var("PLAYMAT_AI_THREADS", "1");
    let repo = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("mana-s2-e2e-ok-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let listener = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = rt.block_on(Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base)).unwrap();
    let alice = rt.block_on(s.new_token(Role::Worker, "alice")).unwrap().token;
    let bob = rt.block_on(s.new_token_with_trust(Role::Worker, "bob", Some(TrustLevel::Trusted))).unwrap().token;
    let trainer = rt.block_on(s.new_token(Role::Trainer, "trainer")).unwrap().token;
    rt.spawn(playmat_swarm::serve(listener, s));

    let http = Client::new();
    let api = |path: &str| format!("{base}/api/v1{path}");
    let unit = test_unit(&repo, "s2-r1-u000", 710_000, 2, "cfg:policy=ismcts+net=builtin+hide=1+iters=4+samples=1");
    let round: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound { project: "test".into(), run: "s2-e2e".into(), round: 1, priority: 0, units: vec![unit] })
        .send()
        .unwrap()
        .json()
        .unwrap();

    // Probation worker plays with 2 jobs; coordinator automatically queues a VerifyMode::Rerun unit
    swarm::run(swarm::Opts {
        coordinator: base.clone(),
        token: alice,
        name: "alice-2t".into(),
        jobs: 2,
        work: tmp.join("alice").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 1,
    })
    .unwrap();

    let mid: RoundStatus =
        http.get(api(&format!("/rounds/{}", round.id))).bearer_auth(&trainer).send().unwrap().json().unwrap();
    assert_eq!(mid.units.get("complete"), Some(&1));
    assert_eq!(mid.verify_pending, 1);

    // Trusted verifier runs with 1 job (different thread count!) -> deal-sorted shards + game hashes match bit-for-bit
    swarm::run(swarm::Opts {
        coordinator: base.clone(),
        token: bob,
        name: "bob-1t".into(),
        jobs: 1,
        work: tmp.join("bob").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 1,
    })
    .unwrap();

    let after: RoundStatus =
        http.get(api(&format!("/rounds/{}", round.id))).bearer_auth(&trainer).send().unwrap().json().unwrap();
    assert_eq!(after.units.get("accepted"), Some(&1));
    assert_eq!(after.verify_pending, 0);

    let lb: Vec<LeaderboardEntry> = http.get(api("/leaderboard")).send().unwrap().json().unwrap();
    assert!(lb.iter().any(|e| e.account == "alice" && e.accepted_units == 1 && e.credit >= 10.0));
    assert!(lb.iter().any(|e| e.account == "bob" && e.verified_units == 1 && e.credit >= 10.0));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn s2_gate_planted_cheaters_caught_by_real_verifier() {
    use crate::{read_json, run_unit};
    use playmat_swarm_proto::ResultManifest;
    use std::sync::atomic::AtomicBool;

    std::env::set_var("PLAYMAT_AI_THREADS", "1");
    let repo = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("mana-s2-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let listener = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = rt.block_on(Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base)).unwrap();
    let trainer = rt.block_on(s.new_token(Role::Trainer, "trainer")).unwrap().token;
    let admin = rt.block_on(s.new_token(Role::Admin, "owner")).unwrap().token;
    let verifier =
        rt.block_on(s.new_token_with_trust(Role::Worker, "verifier", Some(TrustLevel::Trusted))).unwrap().token;
    let cheater_outcome = rt.block_on(s.new_token(Role::Worker, "cheater-outcome")).unwrap();
    let cheater_search = rt.block_on(s.new_token(Role::Worker, "cheater-search")).unwrap();
    let cheater_shard = rt.block_on(s.new_token(Role::Worker, "cheater-shard")).unwrap();
    rt.spawn(playmat_swarm::serve(listener, s));

    let http = Client::new();
    let api = |path: &str| format!("{base}/api/v1{path}");
    let honest_cfg = "cfg:policy=ismcts+net=builtin+hide=1+iters=4+samples=1";
    let units = vec![
        test_unit(&repo, "g-u000", 720_000, 2, honest_cfg),
        test_unit(&repo, "g-u001", 720_002, 2, honest_cfg),
        test_unit(&repo, "g-u002", 720_004, 2, honest_cfg),
    ];
    let round: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound { project: "test".into(), run: "s2-e2e".into(), round: 1, priority: 0, units })
        .send()
        .unwrap()
        .json()
        .unwrap();

    let nets_dir = tmp.join("nets");
    std::fs::create_dir_all(&nets_dir).unwrap();

    // Helper: lease a unit as a cheater, produce a tampered output directory + manifest, upload & complete
    let submit_cheater = |token: &str, name: &str, mode: &str| {
        let h: HelloReply = http
            .post(api("/hello"))
            .bearer_auth(token)
            .json(&Hello {
                name: name.into(),
                build: "dev".into(),
                class: playmat_swarm_proto::determinism_class(STRICT),
                strict: STRICT,
                threads: 1,
                os: "test".into(),
            })
            .send()
            .unwrap()
            .json()
            .unwrap();
        let lease: Lease = http
            .post(api("/lease"))
            .bearer_auth(token)
            .json(&LeaseRequest { host: h.host })
            .send()
            .unwrap()
            .json()
            .unwrap();
        let udir = tmp.join(format!("cheat-{mode}"));
        let out = udir.join("out");
        std::fs::create_dir_all(&udir).unwrap();
        let mut play_unit = lease.unit.clone();
        if mode == "fake-search" {
            // Cheater 2: runs a weaker/faked search (iters=1 instead of iters=4) -> 100% legal games that pass replay!
            play_unit.learner = "cfg:policy=ismcts+net=builtin+hide=1+iters=1+samples=1".into();
        }
        let unit_path = udir.join("unit.json");
        std::fs::write(&unit_path, serde_json::to_string(&play_unit).unwrap()).unwrap();
        run_unit(
            unit_path.to_str().unwrap(),
            out.to_str().unwrap(),
            nets_dir.to_str().unwrap(),
            &repo,
            1,
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut manifest: ResultManifest = read_json(out.join("manifest.json").to_str().unwrap()).unwrap();
        if mode == "forged-outcome" {
            // Cheater 1: flips the reported winner of game 0
            manifest.games[0].winner = match manifest.games[0].winner {
                Some(0) => Some(1),
                _ => Some(0),
            };
        }
        let mut files = Vec::new();
        for (path, sha) in &manifest.files {
            let size = std::fs::metadata(out.join(path)).unwrap().len();
            files.push(FileInfo { path: path.clone(), size, sha256: sha.clone() });
        }
        let urls: Urls = http
            .post(api(&format!("/units/{}/uploads", lease.unit.id)))
            .bearer_auth(token)
            .json(&UploadRequest { files })
            .send()
            .unwrap()
            .json()
            .unwrap();
        for (path, url) in &urls.urls {
            let mut body = std::fs::read(out.join(path)).unwrap();
            if mode == "poison-shard" && path == "w.target.f16" && !body.is_empty() {
                // Cheater 3: honest manifest hashes, but flips a byte in uploaded shard (same size -> passes intake!)
                body[0] ^= 0xff;
            }
            assert!(http
                .put(url)
                .header("content-length", body.len())
                .body(body)
                .send()
                .unwrap()
                .status()
                .is_success());
        }
        let res = http
            .post(api(&format!("/units/{}/complete", lease.unit.id)))
            .bearer_auth(token)
            .json(&Complete { manifest })
            .send()
            .unwrap();
        assert_eq!(res.status(), 204);
    };

    submit_cheater(&cheater_outcome.token, "c1", "forged-outcome");
    submit_cheater(&cheater_search.token, "c2", "fake-search");
    submit_cheater(&cheater_shard.token, "c3", "poison-shard");

    // Real Trusted verifier processes all 3 verify units
    swarm::run(swarm::Opts {
        coordinator: base.clone(),
        token: verifier,
        name: "trusted-ver".into(),
        jobs: 1,
        work: tmp.join("ver").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 3,
    })
    .unwrap();

    // All 3 planted cheaters must be caught on unit 1: 0 accepted, 3 rejected, 3 open inbox items, 0 cheater credit
    let after: RoundStatus =
        http.get(api(&format!("/rounds/{}", round.id))).bearer_auth(&trainer).send().unwrap().json().unwrap();
    assert_eq!(after.units.get("accepted").copied().unwrap_or(0), 0);
    assert_eq!(after.units.get("rejected"), Some(&3));

    let inbox: Vec<InboxItem> =
        http.get(api("/admin/inbox?status=open")).bearer_auth(&admin).send().unwrap().json().unwrap();
    assert_eq!(inbox.len(), 3, "expected 3 open inbox items, got {inbox:?}");
    assert!(inbox.iter().any(|i| i.account.as_deref() == Some("cheater-outcome")));
    assert!(inbox.iter().any(|i| i.account.as_deref() == Some("cheater-search")));
    assert!(inbox.iter().any(|i| i.account.as_deref() == Some("cheater-shard") && i.detail.contains("w.target.f16")));

    let lb: Vec<LeaderboardEntry> = http.get(api("/leaderboard")).send().unwrap().json().unwrap();
    for name in ["cheater-outcome", "cheater-search", "cheater-shard"] {
        let e = lb.iter().find(|e| e.account == name).unwrap();
        assert_eq!(e.accepted_units, 0);
        assert_eq!(e.credit, 0.0);
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

fn kind_unit(repo: &str, id: &str, kind: UnitKind, seed: u64, games: u32, learner: &str) -> WorkUnit {
    let mut u = test_unit(repo, id, seed, games, learner);
    u.kind = kind;
    u.keep = 0;
    u
}

/// S4 Gate: "A swarm gate agrees with a trusted-only gate."
/// Runs the exact same Match deals through (1) a trusted-only coordinator and (2) a mixed swarm coordinator with
/// Reputable + Trusted workers and a planted sandbagger trying to flip unit 0's outcome. Asserts that after
/// sandbagger rejection + reissue + verification, both gates reach identical W/D/L, Elo, LLR and status.
#[test]
fn s4_gate_swarm_agrees_with_trusted_only_gate() {
    use crate::{read_json, run_unit};
    use playmat_swarm_proto::ResultManifest;
    use std::sync::atomic::AtomicBool;

    std::env::set_var("PLAYMAT_AI_THREADS", "1");
    let repo = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("mana-s4-gate-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let learner = "cfg:policy=ismcts+net=builtin+hide=1+iters=4+samples=1";
    let base_sha = "a".repeat(64);
    let cand_sha = "b".repeat(64);
    let sprt = SprtConfig { elo0: 0.0, elo1: 10.0, alpha: 0.05, beta: 0.05, max_share: 1.0, auto_promote: true };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let http = Client::new();

    // ---- 1. Trusted-only gate ----
    let l1 = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let b1 = format!("http://{}", l1.local_addr().unwrap());
    let s1 = rt
        .block_on(Swarm::open(
            tmp.join("trusted.db").to_str().unwrap(),
            Store::Local(Local::new(tmp.join("store1"), b1.clone())),
            &b1,
        ))
        .unwrap();
    let tr1 = rt.block_on(s1.new_token(Role::Trainer, "trainer")).unwrap().token;
    let w_tr = rt.block_on(s1.new_token_with_trust(Role::Worker, "trusted", Some(TrustLevel::Trusted))).unwrap().token;
    rt.spawn(playmat_swarm::serve(l1, s1));

    let units1 = vec![
        kind_unit(&repo, "m-u000", UnitKind::Match, 730_000, 2, learner),
        kind_unit(&repo, "m-u001", UnitKind::Match, 730_002, 2, learner),
    ];
    let g1: GateStatus = http
        .post(format!("{b1}/api/v1/gates"))
        .bearer_auth(&tr1)
        .json(&NewGate {
            project: "test".into(),
            run: "s2-e2e".into(),
            round: 1,
            priority: 10,
            candidate_sha: cand_sha.clone(),
            candidate_label: "cand".into(),
            baseline_sha: base_sha.clone(),
            baseline_label: "base".into(),
            sprt: sprt.clone(),
            units: units1,
        })
        .send()
        .unwrap()
        .json()
        .unwrap();

    swarm::run(swarm::Opts {
        coordinator: b1.clone(),
        token: w_tr,
        name: "trusted-1".into(),
        jobs: 1,
        work: tmp.join("w-trusted").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 2,
    })
    .unwrap();
    for uid in ["m-u000", "m-u001"] {
        assert_eq!(
            http.post(format!("{b1}/api/v1/units/{uid}/verdict"))
                .bearer_auth(&tr1)
                .json(&Verdict { accept: true, reason: "trusted".into() })
                .send()
                .unwrap()
                .status(),
            204
        );
    }
    let trusted_gate: GateStatus = http.get(format!("{b1}/api/v1/gates/{}", g1.id)).send().unwrap().json().unwrap();

    // ---- 2. Swarm gate (Reputable workers + planted sandbagger + Trusted verifier) ----
    let l2 = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let b2 = format!("http://{}", l2.local_addr().unwrap());
    let s2 = rt
        .block_on(Swarm::open(
            tmp.join("swarm.db").to_str().unwrap(),
            Store::Local(Local::new(tmp.join("store2"), b2.clone())),
            &b2,
        ))
        .unwrap();
    let tr2 = rt.block_on(s2.new_token(Role::Trainer, "trainer")).unwrap().token;
    let sandbag =
        rt.block_on(s2.new_token_with_trust(Role::Worker, "sandbagger", Some(TrustLevel::Reputable))).unwrap().token;
    let rep1 = rt.block_on(s2.new_token_with_trust(Role::Worker, "rep-1", Some(TrustLevel::Reputable))).unwrap().token;
    let rep2 = rt.block_on(s2.new_token_with_trust(Role::Worker, "rep-2", Some(TrustLevel::Reputable))).unwrap().token;
    let ver = rt.block_on(s2.new_token_with_trust(Role::Worker, "verifier", Some(TrustLevel::Trusted))).unwrap().token;
    rt.spawn(playmat_swarm::serve(l2, s2));

    let units2 = vec![
        kind_unit(&repo, "m-u000", UnitKind::Match, 730_000, 2, learner),
        kind_unit(&repo, "m-u001", UnitKind::Match, 730_002, 2, learner),
    ];
    let g2: GateStatus = http
        .post(format!("{b2}/api/v1/gates"))
        .bearer_auth(&tr2)
        .json(&NewGate {
            project: "test".into(),
            run: "s2-e2e".into(),
            round: 1,
            priority: 10,
            candidate_sha: cand_sha,
            candidate_label: "cand".into(),
            baseline_sha: base_sha,
            baseline_label: "base".into(),
            sprt,
            units: units2,
        })
        .send()
        .unwrap()
        .json()
        .unwrap();

    // Sandbagger leases m-u000 and flips every winner to try to poison the gate
    let sh: HelloReply = http
        .post(format!("{b2}/api/v1/hello"))
        .bearer_auth(&sandbag)
        .json(&Hello {
            name: "sandbag-box".into(),
            build: "dev".into(),
            class: playmat_swarm_proto::determinism_class(STRICT),
            strict: STRICT,
            threads: 1,
            os: "test".into(),
        })
        .send()
        .unwrap()
        .json()
        .unwrap();
    let sl: Lease = http
        .post(format!("{b2}/api/v1/lease"))
        .bearer_auth(&sandbag)
        .json(&LeaseRequest { host: sh.host })
        .send()
        .unwrap()
        .json()
        .unwrap();
    let sdir = tmp.join("sandbag");
    let sout = sdir.join("out");
    std::fs::create_dir_all(&sdir).unwrap();
    std::fs::create_dir_all(tmp.join("nets")).unwrap();
    std::fs::write(sdir.join("unit.json"), serde_json::to_string(&sl.unit).unwrap()).unwrap();
    run_unit(
        sdir.join("unit.json").to_str().unwrap(),
        sout.to_str().unwrap(),
        tmp.join("nets").to_str().unwrap(),
        &repo,
        1,
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut sm: ResultManifest = read_json(sout.join("manifest.json").to_str().unwrap()).unwrap();
    for g in &mut sm.games {
        g.winner = match g.winner {
            Some(0) => Some(1),
            _ => Some(0),
        };
    }
    let sfiles: Vec<FileInfo> = sm
        .files
        .iter()
        .map(|(p, sha)| FileInfo {
            path: p.clone(),
            size: std::fs::metadata(sout.join(p)).unwrap().len(),
            sha256: sha.clone(),
        })
        .collect();
    let surls: Urls = http
        .post(format!("{b2}/api/v1/units/{}/uploads", sl.unit.id))
        .bearer_auth(&sandbag)
        .json(&UploadRequest { files: sfiles })
        .send()
        .unwrap()
        .json()
        .unwrap();
    for (p, u) in &surls.urls {
        let body = std::fs::read(sout.join(p)).unwrap();
        assert!(http.put(u).header("content-length", body.len()).body(body).send().unwrap().status().is_success());
    }
    assert_eq!(
        http.post(format!("{b2}/api/v1/units/{}/complete", sl.unit.id))
            .bearer_auth(&sandbag)
            .json(&Complete { manifest: sm })
            .send()
            .unwrap()
            .status(),
        204
    );

    // Verifier catches the sandbagger on m-u000; trainer reissues m-u000; honest rep-1 & rep-2 play both units
    swarm::run(swarm::Opts {
        coordinator: b2.clone(),
        token: ver.clone(),
        name: "ver-1".into(),
        jobs: 1,
        work: tmp.join("w-ver1").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 1,
    })
    .unwrap();
    assert_eq!(
        http.post(format!("{b2}/api/v1/admin/units/m-u000/reissue")).bearer_auth(&tr2).send().unwrap().status(),
        204
    );
    // Honest reputable workers play m-u000 and m-u001, each verified bit-for-bit by the trusted verifier
    for (tok, nm, idx) in [(rep1, "rep-1", 1), (rep2, "rep-2", 2)] {
        swarm::run(swarm::Opts {
            coordinator: b2.clone(),
            token: tok,
            name: nm.into(),
            jobs: 1,
            work: tmp.join(format!("w-{nm}")).to_string_lossy().into(),
            repo: repo.clone(),
            idle: false,
            units: 1,
        })
        .unwrap();
        swarm::run(swarm::Opts {
            coordinator: b2.clone(),
            token: ver.clone(),
            name: "ver-1".into(),
            jobs: 1,
            work: tmp.join(format!("w-ver-check-{idx}")).to_string_lossy().into(),
            repo: repo.clone(),
            idle: false,
            units: 1,
        })
        .unwrap();
    }

    let swarm_gate: GateStatus = http.get(format!("{b2}/api/v1/gates/{}", g2.id)).send().unwrap().json().unwrap();
    assert_eq!(
        (swarm_gate.wins, swarm_gate.draws, swarm_gate.losses),
        (trusted_gate.wins, trusted_gate.draws, trusted_gate.losses)
    );
    assert!((swarm_gate.elo - trusted_gate.elo).abs() < 1e-6);
    assert!((swarm_gate.llr - trusted_gate.llr).abs() < 1e-6);
    assert_eq!(swarm_gate.status, trusted_gate.status);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn s4_bench_unit_runs_through_real_worker_and_rates_network() {
    std::env::set_var("PLAYMAT_AI_THREADS", "1");
    let repo = format!("{}/../../..", env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("mana-s4-bench-e2e-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let listener = rt.block_on(tokio::net::TcpListener::bind("127.0.0.1:0")).unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let s = rt
        .block_on(Swarm::open(
            tmp.join("bench.db").to_str().unwrap(),
            Store::Local(Local::new(tmp.join("store"), base.clone())),
            &base,
        ))
        .unwrap();
    let trainer = rt.block_on(s.new_token(Role::Trainer, "trainer")).unwrap().token;
    let rep = rt.block_on(s.new_token_with_trust(Role::Worker, "rep", Some(TrustLevel::Reputable))).unwrap().token;
    let ver = rt.block_on(s.new_token_with_trust(Role::Worker, "ver", Some(TrustLevel::Trusted))).unwrap().token;
    rt.spawn(playmat_swarm::serve(listener, s));

    let http = Client::new();
    let net_sha = "c".repeat(64);
    let unit = kind_unit(
        &repo,
        "b-u000",
        UnitKind::Bench,
        730_000,
        2,
        "cfg:policy=ismcts+net=builtin+hide=1+iters=4+samples=1",
    );
    let bench: BenchStatus = http
        .post(format!("{base}/api/v1/bench"))
        .bearer_auth(&trainer)
        .json(&NewBench {
            project: "test".into(),
            run: "s2-e2e".into(),
            round: 1,
            priority: 5,
            net_sha: net_sha.clone(),
            net_label: "bench-net".into(),
            anchor_elo: 1000.0,
            units: vec![unit],
        })
        .send()
        .unwrap()
        .json()
        .unwrap();
    assert_eq!(bench.games, 0);

    swarm::run(swarm::Opts {
        coordinator: base.clone(),
        token: rep,
        name: "rep-box".into(),
        jobs: 1,
        work: tmp.join("w-rep").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 1,
    })
    .unwrap();
    swarm::run(swarm::Opts {
        coordinator: base.clone(),
        token: ver,
        name: "ver-box".into(),
        jobs: 1,
        work: tmp.join("w-ver").to_string_lossy().into(),
        repo: repo.clone(),
        idle: false,
        units: 1,
    })
    .unwrap();

    let nets: Vec<NetworkInfo> = http.get(format!("{base}/api/v1/networks")).send().unwrap().json().unwrap();
    let net = nets.iter().find(|n| n.sha256 == net_sha).unwrap();
    assert_eq!(net.bench_games, 2);
    assert!(net.bench_elo.is_some());
    assert_eq!(net.anchors.len(), 1);
    assert_eq!(net.anchors[0].opponent, "host@s6");
    let _ = std::fs::remove_dir_all(&tmp);
}
