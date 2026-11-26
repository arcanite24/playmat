//! Mana@Home S4 integration tests (docs/swarm/Design.md):
//! - SPRT early stopping, single-account concentration guard (auto-spawns cross-account re-runs), and auto-promotion.
//! - Admin hold, resume, manual promote (requires written reason if unpassed), and rollback (requires written reason).
//! - Archbench `UnitKind::Bench` rating rounds updating public `/api/v1/networks` anchored Elo and per-anchor scores.

use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{
    sha256_hex, GameSummary, Opponent, Pins, ResultManifest, UnitKind, VerifyMode, WorkUnit, PROTOCOL,
};
use reqwest::StatusCode;
use std::collections::BTreeMap;

fn gate_unit(id: &str, round: u32, kind: UnitKind, seed: u64, games: u32) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "main".into(),
        run: "s4".into(),
        round,
        kind,
        pins: Pins {
            builds: vec![],
            obs: 3,
            suite: "constructed-2".into(),
            suite_sha256: "0".repeat(64),
            strict: true,
        },
        nets: vec![],
        learner: "cfg:policy=ismcts+net=builtin+iters=4".into(),
        opponents: vec![Opponent { config: "host@s6".into(), weight: 1 }],
        seed,
        games,
        keep: 0,
        fast: None,
        deadline: 0,
        cost: 10.0,
        verify: None,
    }
}

async fn hello(http: &reqwest::Client, api: &impl Fn(&str) -> String, token: &str, name: &str) -> HelloReply {
    http.post(api("/hello"))
        .bearer_auth(token)
        .json(&Hello {
            name: name.into(),
            build: "dev".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 4,
            os: "test".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn complete_match(
    http: &reqwest::Client,
    api: &impl Fn(&str) -> String,
    token: &str,
    unit: &WorkUnit,
    wins: u32,
    draws: u32,
) {
    let body = format!("{{\"unit\":\"{}\"}}\n", unit.id).into_bytes();
    let sha = sha256_hex(&body);
    let urls: Urls = http
        .post(api(&format!("/units/{}/uploads", unit.id)))
        .bearer_auth(token)
        .json(&UploadRequest {
            files: vec![FileInfo { path: "answers.jsonl".into(), size: body.len() as u64, sha256: sha.clone() }],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(http.put(&urls.urls["answers.jsonl"]).body(body).send().await.unwrap().status().is_success());

    let mut games = Vec::new();
    for i in 0..unit.games {
        let deal = unit.seed + i as u64;
        let winner = if i < wins {
            Some(0)
        } else if i < wins + draws {
            None
        } else {
            Some(1)
        };
        games.push(GameSummary {
            deal,
            winner,
            learner: 0,
            opponent: 0,
            turns: 12,
            answers: 30,
            recorded: 0,
            policy_entropy: None,
            log_sha256: sha256_hex(format!("log-{deal}").as_bytes()),
            steps_sha256: sha256_hex(format!("steps-{deal}").as_bytes()),
        });
    }
    let res = http
        .post(api(&format!("/units/{}/complete", unit.id)))
        .bearer_auth(token)
        .json(&Complete {
            manifest: ResultManifest {
                protocol: PROTOCOL,
                unit: unit.id.clone(),
                worker: "sim".into(),
                build: "dev".into(),
                class: "strict-1".into(),
                nets: vec![],
                games,
                failed: vec![],
                files: BTreeMap::from([("answers.jsonl".into(), sha)]),
                elapsed_ms: 1000,
            },
        })
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn sprt_gate_concentration_guard_and_auto_promotion() {
    let tmp = std::env::temp_dir().join(format!("mana-s4-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let t1 = s.new_token_with_trust(Role::Worker, "trusted-a", Some(TrustLevel::Trusted)).await.unwrap().token;
    let t2 = s.new_token_with_trust(Role::Worker, "trusted-b", Some(TrustLevel::Trusted)).await.unwrap().token;
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    let base_sha = "1".repeat(64);
    let cand_sha = "2".repeat(64);
    let units: Vec<WorkUnit> =
        (0..6).map(|i| gate_unit(&format!("g1-u{i:03}"), 1, UnitKind::Match, 100 + i * 50, 50)).collect();
    let gate: GateStatus = http
        .post(api("/gates"))
        .bearer_auth(&trainer)
        .json(&NewGate {
            project: "main".into(),
            run: "s4".into(),
            round: 1,
            priority: 20,
            candidate_sha: cand_sha.clone(),
            candidate_label: "gen-2".into(),
            baseline_sha: base_sha.clone(),
            baseline_label: "gen-1".into(),
            sprt: SprtConfig { elo0: 0.0, elo1: 30.0, alpha: 0.05, beta: 0.05, max_share: 0.50, auto_promote: true },
            units,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(gate.status, "running");

    let h1 = hello(&http, &api, &t1, "box-a").await;
    let h2 = hello(&http, &api, &t2, "box-b").await;

    // Single account (trusted-a) plays 2 units of 50 games with 70% win rate -> crosses SPRT upper bound!
    // Verifier t2 replays them (Replay only, since trusted-a has 0% rerun rate).
    for _ in 0..2 {
        let l: Lease = http
            .post(api("/lease"))
            .bearer_auth(&t1)
            .json(&LeaseRequest { host: h1.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(l.unit.kind, UnitKind::Match);
        complete_match(&http, &api, &t1, &l.unit, 35, 2).await;

        let vl: Lease = http
            .post(api("/lease"))
            .bearer_auth(&t2)
            .json(&LeaseRequest { host: h2.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(vl.unit.kind, UnitKind::Verify);
        assert_eq!(vl.unit.verify.as_ref().unwrap().mode, VerifyMode::Replay);
        assert_eq!(
            http.post(api(&format!("/units/{}/verify-complete", vl.unit.id)))
                .bearer_auth(&t2)
                .json(&VerifyReport {
                    mode: VerifyMode::Replay,
                    target_unit: l.unit.id,
                    passed: true,
                    reason: "replayed".into(),
                    mismatches: vec![],
                    file_mismatches: vec![],
                    elapsed_ms: 20,
                })
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
    }

    // SPRT passed early (4 remaining units cancelled), BUT single account played 100% -> concentration-hold!
    let g_mid: GateStatus = http.get(api(&format!("/gates/{}", gate.id))).send().await.unwrap().json().await.unwrap();
    assert_eq!(g_mid.status, "concentration-hold", "got {g_mid:?}");
    assert!(g_mid.llr >= g_mid.upper_bound);
    assert!((g_mid.top_account_share - 1.0).abs() < 1e-6);

    // Remaining queued match units were early-stopped
    let r_st: RoundStatus = http
        .get(api(&format!("/rounds/{}", gate.round_id)))
        .bearer_auth(&trainer)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r_st.units.get("cancelled"), Some(&4));
    assert_eq!(r_st.verify_pending, 1);

    // trusted-a CANNOT lease its own cross-account rerun
    assert_eq!(
        http.post(api("/lease")).bearer_auth(&t1).json(&LeaseRequest { host: h1.host }).send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    // trusted-b leases and passes the spawned cross-account VerifyMode::Rerun -> share drops to 50% -> auto-promotes!
    let rerun_l: Lease = http
        .post(api("/lease"))
        .bearer_auth(&t2)
        .json(&LeaseRequest { host: h2.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(rerun_l.unit.kind, UnitKind::Verify);
    let spec = rerun_l.unit.verify.as_ref().unwrap();
    assert_eq!(spec.mode, VerifyMode::Rerun);
    assert_eq!(
        http.post(api(&format!("/units/{}/verify-complete", rerun_l.unit.id)))
            .bearer_auth(&t2)
            .json(&VerifyReport {
                mode: VerifyMode::Rerun,
                target_unit: spec.target_unit.clone(),
                passed: true,
                reason: "cross-account rerun matched".into(),
                mismatches: vec![],
                file_mismatches: vec![],
                elapsed_ms: 100,
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    let g_done: GateStatus = http.get(api(&format!("/gates/{}", gate.id))).send().await.unwrap().json().await.unwrap();
    assert_eq!(g_done.status, "promoted", "got {g_done:?}");
    assert!((g_done.top_account_share - 0.5).abs() < 1e-6);

    // Public /networks shows gen-2 promoted and gen-1 superseded
    let nets: Vec<NetworkInfo> = http.get(api("/networks")).send().await.unwrap().json().await.unwrap();
    assert_eq!(nets.iter().find(|n| n.sha256 == cand_sha).unwrap().status, "promoted");
    assert_eq!(nets.iter().find(|n| n.sha256 == base_sha).unwrap().status, "superseded");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn admin_hold_manual_promote_rollback_and_archbench_ratings() {
    let tmp = std::env::temp_dir().join(format!("mana-s4-admin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let admin = s.new_token(Role::Admin, "owner").await.unwrap().token;
    let worker = s.new_token_with_trust(Role::Worker, "trusted-w", Some(TrustLevel::Trusted)).await.unwrap().token;
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    let base_sha = "a".repeat(64);
    let cand_sha = "b".repeat(64);
    let gate: GateStatus = http
        .post(api("/gates"))
        .bearer_auth(&trainer)
        .json(&NewGate {
            project: "main".into(),
            run: "s4".into(),
            round: 1,
            priority: 10,
            candidate_sha: cand_sha.clone(),
            candidate_label: "gen-b".into(),
            baseline_sha: base_sha.clone(),
            baseline_label: "gen-a".into(),
            sprt: SprtConfig { elo0: 0.0, elo1: 10.0, alpha: 0.05, beta: 0.05, max_share: 1.0, auto_promote: true },
            units: vec![gate_unit("g-u000", 1, UnitKind::Match, 1, 20)],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Promoting an unpassed gate without a written reason is rejected (400)
    assert_eq!(
        http.post(api(&format!("/admin/gates/{}/promote", gate.id)))
            .bearer_auth(&admin)
            .json(&GateAction { reason: "   ".into() })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );

    // With a written reason, manual promotion succeeds
    let promoted: GateStatus = http
        .post(api(&format!("/admin/gates/{}/promote", gate.id)))
        .bearer_auth(&admin)
        .json(&GateAction { reason: "architecture change approved".into() })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(promoted.status, "promoted");

    // Rollback to baseline requires written reason and restores baseline to promoted
    assert_eq!(
        http.post(api("/admin/networks/rollback"))
            .bearer_auth(&admin)
            .json(&RollbackRequest { target_sha: base_sha.clone(), reason: "regression on OOD".into() })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    // Archbench rating round (`POST /bench`) updates `networks` with anchored Elo
    let mut bench_unit = gate_unit("b-u000", 1, UnitKind::Bench, 500, 40);
    bench_unit.project = "bench".into();
    let bench: BenchStatus = http
        .post(api("/bench"))
        .bearer_auth(&trainer)
        .json(&NewBench {
            project: "bench".into(),
            run: "s4".into(),
            round: 1,
            priority: 5,
            net_sha: base_sha.clone(),
            net_label: "gen-a".into(),
            anchor_elo: 1000.0,
            units: vec![bench_unit],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(bench.games, 0);

    let h = hello(&http, &api, &worker, "bench-box").await;
    let l: Lease = http
        .post(api("/lease"))
        .bearer_auth(&worker)
        .json(&LeaseRequest { host: h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(l.unit.kind, UnitKind::Bench);
    complete_match(&http, &api, &worker, &l.unit, 30, 0).await;
    assert_eq!(
        http.post(api(&format!("/units/{}/verdict", l.unit.id)))
            .bearer_auth(&trainer)
            .json(&Verdict { accept: true, reason: "replayed".into() })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    let nets: Vec<NetworkInfo> = http.get(api("/networks")).send().await.unwrap().json().await.unwrap();
    let gen_a = nets.iter().find(|n| n.sha256 == base_sha).unwrap();
    assert_eq!(gen_a.status, "promoted");
    assert_eq!(gen_a.bench_games, 40);
    // 30/40 = 75% -> +190.85 Elo above anchor 1000.0 -> ~1190.85
    assert!((gen_a.bench_elo.unwrap() - 1190.85).abs() < 1.0, "{:?}", gen_a.bench_elo);
    assert_eq!(gen_a.anchors[0].opponent, "host@s6");

    let trans: TransparencyLog = http.get(api("/transparency")).send().await.unwrap().json().await.unwrap();
    assert!(trans.events.iter().any(|e| e.action == "rollback"));
    assert!(trans.events.iter().any(|e| e.action == "bench"));
    let _ = std::fs::remove_dir_all(&tmp);
}
