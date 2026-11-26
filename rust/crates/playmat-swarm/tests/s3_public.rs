//! Mana@Home S3 public tracker API, teams, contributor profiles, transparency log, CORS, and SSE tests.

use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{sha256_hex, GameSummary, Opponent, Pins, ResultManifest, UnitKind, WorkUnit, PROTOCOL};
use reqwest::StatusCode;
use std::collections::BTreeMap;

fn make_unit(id: &str, seed: u64) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "main".into(),
        run: "s3".into(),
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
        learner: "cfg:policy=ismcts+net=builtin+iters=4".into(),
        opponents: vec![Opponent { config: "host@s6".into(), weight: 1 }],
        seed,
        games: 4,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 25.0,
        verify: None,
    }
}

#[tokio::test]
async fn public_tracker_teams_profiles_transparency_and_cors() {
    let tmp = std::env::temp_dir().join(format!("mana-s3-pub-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let alice = s.new_token(Role::Worker, "alice").await.unwrap().token;
    let bob = s.new_token_with_trust(Role::Worker, "bob", Some(TrustLevel::Trusted)).await.unwrap().token;
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    // CORS preflight works
    let preflight = http.request(reqwest::Method::OPTIONS, api("/status")).send().await.unwrap();
    assert_eq!(preflight.status(), StatusCode::NO_CONTENT);
    assert_eq!(preflight.headers().get("access-control-allow-origin").unwrap(), "*");

    // Alice creates a team and joins it; Bob joins too
    let team: TeamInfo = http
        .post(api("/teams"))
        .bearer_auth(&alice)
        .json(&NewTeam { name: "Azorius Compute".into(), description: "Control players donating CPU".into() })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(team.name, "Azorius Compute");
    assert_eq!(team.members, 1);
    assert_eq!(
        http.post(api(&format!("/teams/{}/join", team.id))).bearer_auth(&bob).send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    // Publish 1 unit, Alice plays it, trainer accepts it
    let r: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound {
            project: "main".into(),
            run: "s3".into(),
            round: 1,
            priority: 5,
            units: vec![make_unit("u1", 1)],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let h: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&alice)
        .json(&Hello {
            name: "alice-mac".into(),
            build: "dev".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 8,
            os: "macos-aarch64".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let l: Lease = http
        .post(api("/lease"))
        .bearer_auth(&alice)
        .json(&LeaseRequest { host: h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let body = b"{\"ok\":true}\n".to_vec();
    let sha = sha256_hex(&body);
    let urls: Urls = http
        .post(api(&format!("/units/{}/uploads", l.unit.id)))
        .bearer_auth(&alice)
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
    let games = (1..=4)
        .map(|d| GameSummary {
            deal: d,
            winner: Some(0),
            learner: 0,
            opponent: 0,
            turns: 10,
            answers: 30,
            recorded: 6,
            policy_entropy: Some(0.8),
            log_sha256: sha256_hex(format!("l{d}").as_bytes()),
            steps_sha256: sha256_hex(format!("s{d}").as_bytes()),
        })
        .collect();
    assert_eq!(
        http.post(api(&format!("/units/{}/complete", l.unit.id)))
            .bearer_auth(&alice)
            .json(&Complete {
                manifest: ResultManifest {
                    protocol: PROTOCOL,
                    unit: l.unit.id.clone(),
                    worker: "alice-mac".into(),
                    build: "dev".into(),
                    class: "strict-1".into(),
                    nets: vec![],
                    games,
                    failed: vec![],
                    files: BTreeMap::from([("answers.jsonl".into(), sha)]),
                    elapsed_ms: 2000,
                },
            })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
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

    // Public status includes platforms, total_decisions, and active_round
    let st: Status = http.get(api("/status")).send().await.unwrap().json().await.unwrap();
    assert_eq!(st.games_accepted, 4);
    assert_eq!(st.total_decisions, 24);
    assert_eq!(st.platforms.get("macos-aarch64"), Some(&1));
    assert_eq!(st.active_round.as_ref().unwrap().id, r.id);

    // Public projects, feed, teams, windowed leaderboard, contributor profile, transparency
    let projects: Vec<PublicRound> = http.get(api("/projects")).send().await.unwrap().json().await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].games, 4);

    let feed: Vec<FeedItem> = http.get(api("/feed")).send().await.unwrap().json().await.unwrap();
    assert_eq!(feed.len(), 1);
    assert_eq!(feed[0].account, "alice");

    let teams: Vec<TeamInfo> = http.get(api("/teams?window=24h")).send().await.unwrap().json().await.unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0].members, 2);
    assert_eq!(teams[0].games, 4);
    assert!(teams[0].credit >= 25.0);

    let lb: Vec<LeaderboardEntry> = http.get(api("/leaderboard?window=7d")).send().await.unwrap().json().await.unwrap();
    assert_eq!(lb[0].account, "alice");
    assert_eq!(lb[0].team.as_deref(), Some("Azorius Compute"));

    let profile: ContributorProfile = http.get(api("/contributors/alice")).send().await.unwrap().json().await.unwrap();
    assert_eq!(profile.accepted_units, 1);
    assert_eq!(profile.team.as_deref(), Some("Azorius Compute"));
    assert!(profile.badges.contains(&"First Spark".to_string()));
    assert_eq!(profile.hosts.len(), 1);
    assert_eq!(profile.recent.len(), 1);

    let trans: TransparencyLog = http.get(api("/transparency")).send().await.unwrap().json().await.unwrap();
    assert_eq!(trans.accepted_units, 1);
    assert!(trans.events.iter().any(|e| e.action == "round"));
    let _ = std::fs::remove_dir_all(&tmp);
}
