//! Mana@Home S6 integration tests (docs/swarm/S6.md):
//! - Matured credit balance (`GET /api/v1/credits/balance`) and Playmat cosmetics bridge redemption
//!   (`POST /api/v1/bridge/redeem`), including secret validation, maturation gating, and double-spend protection.
//! - Live Watch (`GET /api/v1/watch` and `GET /api/v1/watch/{unit}/{deal}`) returning featured games and answer logs.

use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{sha256_hex, GameSummary, Opponent, Pins, ResultManifest, UnitKind, WorkUnit, PROTOCOL};
use reqwest::StatusCode;
use std::collections::BTreeMap;

fn sample_unit(id: &str) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "main".into(),
        run: "s6".into(),
        round: 1,
        kind: UnitKind::Selfplay,
        pins: Pins {
            builds: vec![],
            obs: 3,
            suite: "constructed-2".into(),
            suite_sha256: "0".repeat(64),
            strict: true,
        },
        nets: vec![],
        learner: "cfg:policy=quick".into(),
        opponents: vec![Opponent { config: "host@s6".into(), weight: 1 }],
        seed: 900,
        games: 2,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 150.0,
        verify: None,
    }
}

#[tokio::test]
async fn credits_bridge_redeems_matured_once_and_live_watch_serves_answer_log() {
    std::env::set_var("SWARM_BRIDGE_SECRET", "test-bridge-secret-s6");
    let tmp = std::env::temp_dir().join(format!("mana-s6-reach-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();

    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let worker = s.new_token_with_trust(Role::Worker, "alice", Some(TrustLevel::Trusted)).await.unwrap().token;
    let db = s.db.clone();
    tokio::spawn(playmat_swarm::serve(listener, s));

    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    // Publish and complete one unit with a real answers.jsonl entry
    assert_eq!(
        http.post(api("/rounds"))
            .bearer_auth(&trainer)
            .json(&NewRound {
                project: "main".into(),
                run: "s6".into(),
                round: 1,
                priority: 5,
                units: vec![sample_unit("s6-u000")],
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let h: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&worker)
        .json(&Hello {
            name: "alice-pc".into(),
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
    let lease: Lease = http
        .post(api("/lease"))
        .bearer_auth(&worker)
        .json(&LeaseRequest { host: h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let answers_body = concat!(
        "{\"deal\":900,\"log_sha256\":\"",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "\",\"log\":\"0m1 1m1 0A0 1A1 0O3\"}\n",
        "{\"deal\":901,\"log_sha256\":\"",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "\",\"log\":\"0m1 1m1 0A0\"}\n"
    )
    .as_bytes()
    .to_vec();
    let sha = sha256_hex(&answers_body);
    let urls: Urls = http
        .post(api(&format!("/units/{}/uploads", lease.unit.id)))
        .bearer_auth(&worker)
        .json(&UploadRequest {
            files: vec![FileInfo {
                path: "answers.jsonl".into(),
                size: answers_body.len() as u64,
                sha256: sha.clone(),
            }],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(http.put(&urls.urls["answers.jsonl"]).body(answers_body).send().await.unwrap().status().is_success());

    assert_eq!(
        http.post(api(&format!("/units/{}/complete", lease.unit.id)))
            .bearer_auth(&worker)
            .json(&Complete {
                manifest: ResultManifest {
                    protocol: PROTOCOL,
                    unit: lease.unit.id.clone(),
                    worker: "alice-pc".into(),
                    build: "dev".into(),
                    class: "strict-1".into(),
                    nets: vec![],
                    games: vec![
                        GameSummary {
                            deal: 900,
                            winner: Some(0),
                            learner: 0,
                            opponent: 0,
                            turns: 8,
                            answers: 5,
                            recorded: 2,
                            policy_entropy: Some(0.8),
                            log_sha256: "a".repeat(64),
                            steps_sha256: "c".repeat(64),
                        },
                        GameSummary {
                            deal: 901,
                            winner: Some(1),
                            learner: 1,
                            opponent: 0,
                            turns: 6,
                            answers: 3,
                            recorded: 1,
                            policy_entropy: Some(0.7),
                            log_sha256: "b".repeat(64),
                            steps_sha256: "d".repeat(64),
                        },
                    ],
                    failed: vec![],
                    files: BTreeMap::from([("answers.jsonl".into(), sha)]),
                    elapsed_ms: 400,
                },
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        http.post(api(&format!("/units/{}/verdict", lease.unit.id)))
            .bearer_auth(&trainer)
            .json(&Verdict { accept: true, reason: "verified".into() })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    // 1. Live Watch lists both games and decodes deal 900's answer log tokens
    let watch: Vec<WatchGame> = http.get(api("/watch")).send().await.unwrap().json().await.unwrap();
    assert_eq!(watch.len(), 2);
    let replay: WatchReplay = http.get(api("/watch/s6-u000/900")).send().await.unwrap().json().await.unwrap();
    assert_eq!(replay.game.deal, 900);
    assert_eq!(replay.tokens, vec!["0m1", "1m1", "0A0", "1A1", "0O3"]);

    // 2. Pending credit (not yet matured) gives 0 redeemable tokens
    let bal0: CreditBalance =
        http.get(api("/credits/balance")).bearer_auth(&worker).send().await.unwrap().json().await.unwrap();
    assert!(bal0.credit_pending >= 150.0);
    assert_eq!(bal0.redeemable_tokens, 0);

    // Wrong bridge secret is rejected (401)
    assert_eq!(
        http.post(api("/bridge/redeem"))
            .header("x-swarm-bridge-secret", "wrong")
            .json(&BridgeRedeemRequest { worker_token: worker.clone(), playmat_user: "Alice".into() })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );

    // Fast-forward credit maturation past the 7-day window
    sqlx::query("UPDATE credits SET mature_at = 1").execute(&db).await.unwrap();
    let bal1: CreditBalance =
        http.get(api("/credits/balance")).bearer_auth(&worker).send().await.unwrap().json().await.unwrap();
    assert!(bal1.credit_matured >= 150.0);
    assert!(bal1.redeemable_tokens >= 1);

    // First redemption converts matured credit into Playmat capsule tokens
    let red1: BridgeRedeemReply = http
        .post(api("/bridge/redeem"))
        .header("x-swarm-bridge-secret", "test-bridge-secret-s6")
        .json(&BridgeRedeemRequest { worker_token: worker.clone(), playmat_user: "Alice".into() })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(red1.account, "alice");
    assert_eq!(red1.capsule_tokens, bal1.redeemable_tokens);
    assert!(red1.redemption_id.is_some());

    // Immediate second redemption yields 0 tokens (no double spend)
    let red2: BridgeRedeemReply = http
        .post(api("/bridge/redeem"))
        .header("x-swarm-bridge-secret", "test-bridge-secret-s6")
        .json(&BridgeRedeemRequest { worker_token: worker.clone(), playmat_user: "Alice".into() })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(red2.capsule_tokens, 0);
    assert!(red2.redemption_id.is_none());

    let _ = std::fs::remove_dir_all(&tmp);
}
