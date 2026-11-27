use playmat_swarm::store::{Local, Store};
use playmat_swarm::{serve, Swarm};
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{sha256_hex, GameSummary, Opponent, Pins, ResultManifest, UnitKind, WorkUnit, PROTOCOL};
use reqwest::StatusCode;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn tmp_dir(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("playmat-swarm-s7-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn test_unit(id: &str) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "mana".into(),
        run: "drain".into(),
        round: 1,
        kind: UnitKind::Selfplay,
        pins: Pins { builds: vec![], obs: 3, suite: "std".into(), suite_sha256: "a".repeat(64), strict: true },
        nets: vec![],
        learner: "cfg:policy=quick".into(),
        opponents: vec![Opponent { config: "self".into(), weight: 1 }],
        seed: 100,
        games: 1,
        keep: 0,
        fast: None,
        deadline: 0,
        cost: 1.0,
        verify: None,
    }
}

#[tokio::test]
async fn root_redirects_to_tracker_and_purge_intake_deletes_trained_files() {
    let dir = tmp_dir("ops");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let store = Store::Local(Local::new(dir.join("store"), base.clone()));
    let swarm = Swarm::open(dir.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();

    let trainer = swarm.new_token(Role::Trainer, "trainer").await.unwrap();
    let worker = swarm.new_token_with_trust(Role::Worker, "alice", Some(TrustLevel::Trusted)).await.unwrap();
    let server = tokio::spawn(serve(listener, swarm));

    // 1. GET / redirects to the tracker URL instead of 404.
    let no_redir = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let res = no_redir.get(format!("{base}/")).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.contains("/swarm"), "unexpected redirect target: {loc}");

    // 2. Publish a round, complete & accept a unit with an uploaded file in local store.
    let client = reqwest::Client::new();
    let round: RoundStatus = client
        .post(format!("{base}/api/v1/rounds"))
        .bearer_auth(&trainer.token)
        .json(&NewRound {
            project: "mana".into(),
            run: "drain".into(),
            round: 1,
            priority: 1,
            units: vec![test_unit("u-drain-1")],
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let hello: HelloReply = client
        .post(format!("{base}/api/v1/hello"))
        .bearer_auth(&worker.token)
        .json(&Hello {
            name: "host-a".into(),
            build: "test".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 2,
            os: "linux".into(),
            gpu: None,
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();

    let lease: Lease = client
        .post(format!("{base}/api/v1/lease"))
        .bearer_auth(&worker.token)
        .json(&LeaseRequest { host: hello.host })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let unit = lease.unit;

    let payload = b"{\"deal\":100,\"log\":\"1m0\"}\n";
    let sha = sha256_hex(payload);
    let ups: Urls = client
        .post(format!("{base}/api/v1/units/{}/uploads", unit.id))
        .bearer_auth(&worker.token)
        .json(&UploadRequest {
            files: vec![FileInfo { path: "answers.jsonl".into(), size: payload.len() as u64, sha256: sha.clone() }],
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    client.put(&ups.urls["answers.jsonl"]).body(payload.to_vec()).send().await.unwrap().error_for_status().unwrap();

    let mut files = BTreeMap::new();
    files.insert("answers.jsonl".into(), sha);
    let manifest = ResultManifest {
        protocol: PROTOCOL,
        unit: unit.id.clone(),
        worker: "host-a".into(),
        build: "test".into(),
        class: "strict-1".into(),
        nets: vec![],
        games: vec![GameSummary {
            deal: 100,
            opponent: 0,
            learner: 0,
            winner: Some(0),
            turns: 5,
            answers: 10,
            recorded: 0,
            log_sha256: "c".repeat(64),
            steps_sha256: String::new(),
            policy_entropy: None,
        }],
        failed: vec![],
        files,
        elapsed_ms: 500,
    };
    client
        .post(format!("{base}/api/v1/units/{}/complete", unit.id))
        .bearer_auth(&worker.token)
        .json(&Complete { manifest })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    client
        .post(format!("{base}/api/v1/units/{}/verdict", unit.id))
        .bearer_auth(&trainer.token)
        .json(&Verdict { accept: true, reason: "ok".into() })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let stored_path = dir.join(format!("store/intake/{}/{}/answers.jsonl", round.id, unit.id));
    assert!(stored_path.exists(), "expected uploaded file on disk before purge");

    // 3. Purging an untrained round without force=true is refused (409 Conflict).
    let res = client
        .post(format!("{base}/api/v1/admin/rounds/{}/purge-intake", round.id))
        .bearer_auth(&trainer.token)
        .json(&PurgeIntakeRequest::default())
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert!(stored_path.exists());

    // 4. Mark trained, then purge intake -> file deleted cleanly.
    client
        .post(format!("{base}/api/v1/admin/rounds/{}/trained", round.id))
        .bearer_auth(&trainer.token)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let purged: PurgeIntakeReply = client
        .post(format!("{base}/api/v1/admin/rounds/{}/purge-intake", round.id))
        .bearer_auth(&trainer.token)
        .json(&PurgeIntakeRequest {
            force: false,
            include_rejected: false,
            archive_ref: "hf://datasets/nerijs/playmat-ai-selfplay/data/mana-drain-r1".into(),
        })
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(purged.round_id, round.id);
    assert_eq!(purged.units_purged, 1);
    assert_eq!(purged.files_deleted, 1);
    assert!(!stored_path.exists(), "expected intake file removed after purge");

    server.abort();
    let _ = std::fs::remove_dir_all(&dir);
}
