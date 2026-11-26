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
