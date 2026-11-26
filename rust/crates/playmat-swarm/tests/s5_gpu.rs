//! Mana@Home S5 integration tests (docs/swarm/S5.md):
//! - GPU hosts report `gpu: Some(...)` in `/hello` and are counted in `GET /api/v1/status` (`gpu_hosts`).
//! - `UnitKind::SelfplayGpu` units are leased exclusively to GPU hosts (CPU-only hosts get 204).
//! - Completed `SelfplayGpu` units schedule deterministic `VerifyMode::Replay` (not CPU bit-exact re-run)
//!   so any reputable/trusted verifier can verify legality, outcomes, and hashes and award credit.

use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{
    sha256_hex, GameSummary, Opponent, Pins, ResultManifest, UnitKind, VerifyMode, WorkUnit, PROTOCOL,
};
use reqwest::StatusCode;
use std::collections::BTreeMap;

fn gpu_unit(id: &str) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "large".into(),
        run: "s5".into(),
        round: 1,
        kind: UnitKind::SelfplayGpu,
        pins: Pins {
            builds: vec![],
            obs: 3,
            suite: "constructed-2".into(),
            suite_sha256: "0".repeat(64),
            strict: false,
        },
        nets: vec![],
        learner: "cfg:policy=ismcts+net=builtin+iters=16+vloss=8".into(),
        opponents: vec![Opponent { config: "self".into(), weight: 1 }],
        seed: 500,
        games: 4,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 20.0,
        verify: None,
    }
}

#[tokio::test]
async fn gpu_units_lease_only_to_gpu_hosts_and_verify_via_replay() {
    let tmp = std::env::temp_dir().join(format!("mana-s5-gpu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();

    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let cpu_tok = s.new_token_with_trust(Role::Worker, "cpu-vol", Some(TrustLevel::Normal)).await.unwrap().token;
    let gpu_tok = s.new_token_with_trust(Role::Worker, "gpu-vol", Some(TrustLevel::Probation)).await.unwrap().token;
    let ver_tok = s.new_token_with_trust(Role::Worker, "verifier", Some(TrustLevel::Trusted)).await.unwrap().token;
    tokio::spawn(playmat_swarm::serve(listener, s));

    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    // Publish a SelfplayGpu round
    assert_eq!(
        http.post(api("/rounds"))
            .bearer_auth(&trainer)
            .json(&NewRound {
                project: "large".into(),
                run: "s5".into(),
                round: 1,
                priority: 10,
                units: vec![gpu_unit("gpu-u000")],
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // CPU-only host cannot lease SelfplayGpu
    let cpu_h: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&cpu_tok)
        .json(&Hello {
            name: "cpu-box".into(),
            build: "dev".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 8,
            os: "linux-x86_64".into(),
            gpu: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        http.post(api("/lease"))
            .bearer_auth(&cpu_tok)
            .json(&LeaseRequest { host: cpu_h.host })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    // GPU host registers, shows up in `status.gpu_hosts`, and leases the SelfplayGpu unit
    let gpu_h: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&gpu_tok)
        .json(&Hello {
            name: "rtx-4080".into(),
            build: "dev".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 16,
            os: "linux-x86_64".into(),
            gpu: Some("cuda:RTX 4080".into()),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let st: Status = http.get(api("/status")).send().await.unwrap().json().await.unwrap();
    assert_eq!(st.gpu_hosts, 1);

    let lease: Lease = http
        .post(api("/lease"))
        .bearer_auth(&gpu_tok)
        .json(&LeaseRequest { host: gpu_h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(lease.unit.kind, UnitKind::SelfplayGpu);

    // Upload and complete the GPU unit
    let body = b"{\"deal\":500}\n".to_vec();
    let sha = sha256_hex(&body);
    let urls: Urls = http
        .post(api(&format!("/units/{}/uploads", lease.unit.id)))
        .bearer_auth(&gpu_tok)
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

    let games = (500..504)
        .map(|deal| GameSummary {
            deal,
            winner: Some(0),
            learner: 0,
            opponent: 0,
            turns: 10,
            answers: 40,
            recorded: 8,
            policy_entropy: Some(0.9),
            log_sha256: "a".repeat(64),
            steps_sha256: "b".repeat(64),
        })
        .collect();
    assert_eq!(
        http.post(api(&format!("/units/{}/complete", lease.unit.id)))
            .bearer_auth(&gpu_tok)
            .json(&Complete {
                manifest: ResultManifest {
                    protocol: PROTOCOL,
                    unit: lease.unit.id.clone(),
                    worker: "rtx-4080".into(),
                    build: "dev".into(),
                    class: "strict-1".into(),
                    nets: vec![],
                    games,
                    failed: vec![],
                    files: BTreeMap::from([("answers.jsonl".into(), sha)]),
                    elapsed_ms: 500,
                },
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    // Trusted CPU verifier receives a Replay verify unit (NOT bit-exact Rerun, even though gpu-vol is Probation!)
    let ver_h: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&ver_tok)
        .json(&Hello {
            name: "ver-cpu".into(),
            build: "dev".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 4,
            os: "linux-x86_64".into(),
            gpu: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let vlease: Lease = http
        .post(api("/lease"))
        .bearer_auth(&ver_tok)
        .json(&LeaseRequest { host: ver_h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(vlease.unit.kind, UnitKind::Verify);
    assert_eq!(vlease.unit.verify.as_ref().unwrap().mode, VerifyMode::Replay);

    assert_eq!(
        http.post(api(&format!("/units/{}/verify-complete", vlease.unit.id)))
            .bearer_auth(&ver_tok)
            .json(&VerifyReport {
                mode: VerifyMode::Replay,
                target_unit: lease.unit.id.clone(),
                passed: true,
                reason: "replayed 4 games cleanly".into(),
                mismatches: vec![],
                file_mismatches: vec![],
                elapsed_ms: 50,
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    let profile: ContributorProfile =
        http.get(api("/contributors/gpu-vol")).send().await.unwrap().json().await.unwrap();
    assert_eq!(profile.accepted_units, 1);
    assert!(profile.credit > 0.0);
    let _ = std::fs::remove_dir_all(&tmp);
}
