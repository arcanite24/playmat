//! Mana@Home S2 integration tests (docs/swarm/Design.md):
//! - Trust-tier leasing rules (probation/normal can't take verify or match; verifier can never verify its own account; banned gets 403).
//! - Probation (20 accepted units per account AND per host) -> Normal auto-promotion.
//! - Folding@home credit + quick-return bonus + leaderboard + admin credit adjustments.
//! - Post-probation sleeper cheater flagged by per-worker statistics -> forced 100% rerun -> caught within 20 units,
//!   untrained units quarantined, last 30 days of credit frozen, already-trained rounds noted in the admin inbox.
//! - Admin inbox resolution (`false-positive` restores units & unfreezes credits; `confirm-cheat` bans & revokes).

use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{
    sha256_hex, GameSummary, Opponent, Pins, ResultManifest, UnitKind, VerifyMode, WorkUnit, PROTOCOL,
};
use reqwest::StatusCode;
use std::collections::BTreeMap;

fn make_unit(id: &str, round: u32, kind: UnitKind, seed: u64, games: u32, cost: f64) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "test".into(),
        run: "s2".into(),
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
        keep: 200,
        fast: None,
        deadline: 0,
        cost,
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
            gpu: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[derive(Clone, Copy)]
struct SimStats {
    win_count: u32,
    turns: u32,
    entropy: f32,
    elapsed_ms: u64,
}

/// Uploads a dummy `answers.jsonl` and completes a leased unit with the given game stats.
async fn upload_and_complete(
    http: &reqwest::Client,
    api: &impl Fn(&str) -> String,
    token: &str,
    unit: &WorkUnit,
    st: SimStats,
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
        let winner = if i < st.win_count { Some(0) } else { Some(1) };
        games.push(GameSummary {
            deal,
            winner,
            learner: 0,
            opponent: 0,
            turns: st.turns,
            answers: 40,
            recorded: 8,
            policy_entropy: Some(st.entropy),
            log_sha256: sha256_hex(format!("log-{deal}").as_bytes()),
            steps_sha256: sha256_hex(format!("steps-{deal}").as_bytes()),
        });
    }
    let manifest = ResultManifest {
        protocol: PROTOCOL,
        unit: unit.id.clone(),
        worker: "sim".into(),
        build: "dev".into(),
        class: "strict-1".into(),
        nets: vec![],
        games,
        failed: vec![],
        files: BTreeMap::from([("answers.jsonl".into(), sha)]),
        elapsed_ms: st.elapsed_ms,
    };
    let res = http
        .post(api(&format!("/units/{}/complete", unit.id)))
        .bearer_auth(token)
        .json(&Complete { manifest })
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT, "{}", res.text().await.unwrap());
}

#[tokio::test]
async fn trust_probation_promotion_and_host_probation() {
    let tmp = std::env::temp_dir().join(format!("mana-s2-prob-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let admin = s.new_token(Role::Admin, "owner").await.unwrap().token;
    let alice = s.new_token(Role::Worker, "alice").await.unwrap();
    let verifier = s.new_token_with_trust(Role::Worker, "verifier", Some(TrustLevel::Trusted)).await.unwrap();
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    // Publish 21 selfplay units + 1 match unit
    let mut units: Vec<WorkUnit> =
        (0..21).map(|i| make_unit(&format!("r1-u{i:03}"), 1, UnitKind::Selfplay, 100 + i * 10, 4, 10.0)).collect();
    units.push(make_unit("r1-gate", 1, UnitKind::Match, 5000, 4, 10.0));
    let r1: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound { project: "test".into(), run: "s2".into(), round: 1, priority: 0, units })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r1.units.get("queued"), Some(&22));

    let alice_h1 = hello(&http, &api, &alice.token, "alice-pc1").await;
    let alice_h2 = hello(&http, &api, &alice.token, "alice-pc2").await;
    let ver_h = hello(&http, &api, &verifier.token, "ver-box").await;

    // Probation account plays 20 units on host 1: every single one must spawn a Rerun verify unit
    for _ in 0..20 {
        let l: Lease = http
            .post(api("/lease"))
            .bearer_auth(&alice.token)
            .json(&LeaseRequest { host: alice_h1.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(l.unit.kind, UnitKind::Selfplay);
        upload_and_complete(
            &http,
            &api,
            &alice.token,
            &l.unit,
            SimStats { win_count: 2, turns: 12, entropy: 0.85, elapsed_ms: 4000 },
        )
        .await;

        // alice cannot lease verify units (probation + same account)
        // verifier leases the verify unit, confirms it's Rerun, and passes it
        let vl: Lease = http
            .post(api("/lease"))
            .bearer_auth(&verifier.token)
            .json(&LeaseRequest { host: ver_h.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(vl.unit.kind, UnitKind::Verify);
        let spec = vl.unit.verify.as_ref().unwrap();
        assert_eq!(spec.mode, VerifyMode::Rerun);
        assert_eq!(spec.target_unit, l.unit.id);
        assert!(spec.files.contains_key("answers.jsonl"));
        let res = http
            .post(api(&format!("/units/{}/verify-complete", vl.unit.id)))
            .bearer_auth(&verifier.token)
            .json(&VerifyReport {
                mode: VerifyMode::Rerun,
                target_unit: l.unit.id.clone(),
                passed: true,
                reason: "ok".into(),
                mismatches: vec![],
                file_mismatches: vec![],
                elapsed_ms: 100,
            })
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    // After 20 accepted units, alice is automatically promoted to Normal, and host 1 passed probation
    let workers: Vec<WorkerInfo> =
        http.get(api("/admin/workers")).bearer_auth(&admin).send().await.unwrap().json().await.unwrap();
    let aw = workers.iter().find(|w| w.id == alice.id).unwrap();
    assert_eq!(aw.trust, TrustLevel::Normal);
    assert_eq!(aw.accepted_units, 20);
    assert!(aw.credit_pending >= 200.0); // 20 * 10.0 * QRB (up to 1.5x = 300.0)
    assert!(aw.hosts.iter().find(|h| h.id == alice_h1.host).unwrap().probation_passed);
    assert!(!aw.hosts.iter().find(|h| h.id == alice_h2.host).unwrap().probation_passed);

    // Host 2 on the Normal account is STILL in host probation -> unit 21 must get VerifyMode::Rerun
    let l: Lease = http
        .post(api("/lease"))
        .bearer_auth(&alice.token)
        .json(&LeaseRequest { host: alice_h2.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(l.unit.id, "r1-u020");
    upload_and_complete(
        &http,
        &api,
        &alice.token,
        &l.unit,
        SimStats { win_count: 2, turns: 12, entropy: 0.85, elapsed_ms: 4000 },
    )
    .await;
    let vl: Lease = http
        .post(api("/lease"))
        .bearer_auth(&verifier.token)
        .json(&LeaseRequest { host: ver_h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(vl.unit.verify.as_ref().unwrap().mode, VerifyMode::Rerun);
    let _ = http
        .post(api(&format!("/units/{}/verify-complete", vl.unit.id)))
        .bearer_auth(&verifier.token)
        .json(&VerifyReport {
            mode: VerifyMode::Rerun,
            target_unit: l.unit.id,
            passed: true,
            reason: "ok".into(),
            mismatches: vec![],
            file_mismatches: vec![],
            elapsed_ms: 100,
        })
        .send()
        .await
        .unwrap();

    // Only r1-gate (UnitKind::Match) remains: Normal account CANNOT lease it; Trusted verifier CAN
    assert_eq!(
        http.post(api("/lease"))
            .bearer_auth(&alice.token)
            .json(&LeaseRequest { host: alice_h1.host })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    let gate_l: Lease = http
        .post(api("/lease"))
        .bearer_auth(&verifier.token)
        .json(&LeaseRequest { host: ver_h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(gate_l.unit.id, "r1-gate");
    upload_and_complete(
        &http,
        &api,
        &verifier.token,
        &gate_l.unit,
        SimStats { win_count: 2, turns: 12, entropy: 0.85, elapsed_ms: 4000 },
    )
    .await;

    // Verifier CANNOT verify its own account's unit
    assert_eq!(
        http.post(api("/lease"))
            .bearer_auth(&verifier.token)
            .json(&LeaseRequest { host: ver_h.host })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    // Public leaderboard lists both workers with credit
    let lb: Vec<LeaderboardEntry> = http.get(api("/leaderboard")).send().await.unwrap().json().await.unwrap();
    assert_eq!(lb.len(), 2);
    assert!(lb.iter().any(|e| e.account == "alice" && e.accepted_units == 21 && e.credit >= 210.0));
    assert!(lb.iter().any(|e| e.account == "verifier" && e.verified_units == 21 && e.credit >= 210.0));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn sleeper_cheater_caught_within_20_units_and_quarantined() {
    let tmp = std::env::temp_dir().join(format!("mana-s2-sleeper-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let admin = s.new_token(Role::Admin, "owner").await.unwrap().token;
    let honest = s.new_token_with_trust(Role::Worker, "honest", Some(TrustLevel::Normal)).await.unwrap();
    let sleeper = s.new_token_with_trust(Role::Worker, "sleeper", Some(TrustLevel::Normal)).await.unwrap();
    let verifier = s.new_token_with_trust(Role::Worker, "verifier", Some(TrustLevel::Trusted)).await.unwrap();
    // Pre-mark honest & sleeper hosts as past host probation to simulate post-probation Normal workers
    let db = s.db.clone();
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    let honest_h = hello(&http, &api, &honest.token, "honest-1").await;
    let sleeper_h = hello(&http, &api, &sleeper.token, "sleeper-1").await;
    let ver_h = hello(&http, &api, &verifier.token, "ver-1").await;
    sqlx::query("UPDATE hosts SET probation_passed = 1 WHERE id IN (?, ?)")
        .bind(honest_h.host)
        .bind(sleeper_h.host)
        .execute(&db)
        .await
        .unwrap();

    // Round 1 (already trained): sleeper contributes 1 honest unit, honest contributes 8 baseline units
    let r1_units: Vec<WorkUnit> =
        (0..9).map(|i| make_unit(&format!("r1-u{i:03}"), 1, UnitKind::Selfplay, 1000 + i * 20, 20, 10.0)).collect();
    let r1: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound { project: "test".into(), run: "s2".into(), round: 1, priority: 0, units: r1_units })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // 8 honest baseline units
    for _ in 0..8 {
        let l: Lease = http
            .post(api("/lease"))
            .bearer_auth(&honest.token)
            .json(&LeaseRequest { host: honest_h.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        upload_and_complete(
            &http,
            &api,
            &honest.token,
            &l.unit,
            SimStats { win_count: 10, turns: 12, entropy: 0.85, elapsed_ms: 60_000 },
        )
        .await;
        let vl: Lease = http
            .post(api("/lease"))
            .bearer_auth(&verifier.token)
            .json(&LeaseRequest { host: ver_h.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let _ = http
            .post(api(&format!("/units/{}/verify-complete", vl.unit.id)))
            .bearer_auth(&verifier.token)
            .json(&VerifyReport {
                mode: vl.unit.verify.as_ref().unwrap().mode,
                target_unit: l.unit.id,
                passed: true,
                reason: "ok".into(),
                mismatches: vec![],
                file_mismatches: vec![],
                elapsed_ms: 50,
            })
            .send()
            .await
            .unwrap();
    }
    // 1 honest unit from sleeper in round 1
    let l: Lease = http
        .post(api("/lease"))
        .bearer_auth(&sleeper.token)
        .json(&LeaseRequest { host: sleeper_h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    upload_and_complete(
        &http,
        &api,
        &sleeper.token,
        &l.unit,
        SimStats { win_count: 10, turns: 12, entropy: 0.85, elapsed_ms: 60_000 },
    )
    .await;
    let vl: Lease = http
        .post(api("/lease"))
        .bearer_auth(&verifier.token)
        .json(&LeaseRequest { host: ver_h.host })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let _ = http
        .post(api(&format!("/units/{}/verify-complete", vl.unit.id)))
        .bearer_auth(&verifier.token)
        .json(&VerifyReport {
            mode: vl.unit.verify.as_ref().unwrap().mode,
            target_unit: l.unit.id,
            passed: true,
            reason: "ok".into(),
            mismatches: vec![],
            file_mismatches: vec![],
            elapsed_ms: 50,
        })
        .send()
        .await
        .unwrap();
    // Mark round 1 as trained!
    assert_eq!(
        http.post(api(&format!("/admin/rounds/{}/trained", r1.id)))
            .bearer_auth(&trainer)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );

    // Round 2 (untrained): sleeper starts cheating with fake one-hot policies (entropy 0.02) and 100x faster fake search!
    let r2_units: Vec<WorkUnit> =
        (0..20).map(|i| make_unit(&format!("r2-u{i:03}"), 2, UnitKind::Selfplay, 2000 + i * 20, 20, 10.0)).collect();
    let r2: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound { project: "test".into(), run: "s2".into(), round: 2, priority: 0, units: r2_units })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut caught_at: Option<usize> = None;
    for step in 1..=20 {
        let l: Lease = http
            .post(api("/lease"))
            .bearer_auth(&sleeper.token)
            .json(&LeaseRequest { host: sleeper_h.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        // Cheater produces legal games (passes Replay!) with faked search (entropy 0.02, 500ms total)
        upload_and_complete(
            &http,
            &api,
            &sleeper.token,
            &l.unit,
            SimStats { win_count: 10, turns: 12, entropy: 0.02, elapsed_ms: 500 },
        )
        .await;

        let vl: Lease = http
            .post(api("/lease"))
            .bearer_auth(&verifier.token)
            .json(&LeaseRequest { host: ver_h.host })
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let mode = vl.unit.verify.as_ref().unwrap().mode;
        // Replay passes (legal games), Rerun catches the faked search!
        let passed = mode == VerifyMode::Replay;
        let _ = http
            .post(api(&format!("/units/{}/verify-complete", vl.unit.id)))
            .bearer_auth(&verifier.token)
            .json(&VerifyReport {
                mode,
                target_unit: l.unit.id.clone(),
                passed,
                reason: if passed { "replay ok".into() } else { "steps_sha256 mismatch".into() },
                mismatches: if passed { vec![] } else { vec![l.unit.seed] },
                file_mismatches: vec![],
                elapsed_ms: 50,
            })
            .send()
            .await
            .unwrap();
        if !passed {
            caught_at = Some(step);
            break;
        }
    }

    // S2 Gate: sleeper cheater MUST be caught well within 20 units (statistical flag fires at unit 3 -> forced rerun catches on unit 3!)
    let caught = caught_at.expect("sleeper cheater must be caught within 20 units");
    assert!(caught <= 20, "caught at unit {caught}, expected <= 20");

    // All of sleeper's untrained round-2 units must now be rejected or quarantined (0 accepted!)
    let r2_after: RoundStatus =
        http.get(api(&format!("/rounds/{}", r2.id))).bearer_auth(&trainer).send().await.unwrap().json().await.unwrap();
    assert_eq!(r2_after.units.get("accepted").copied().unwrap_or(0), 0);
    assert_eq!(r2_after.units.get("rejected").copied().unwrap_or(0), 1);
    assert_eq!(r2_after.units.get("quarantined").copied().unwrap_or(0), (caught - 1) as u32);

    // Round 1 (already trained) stayed accepted, and the inbox item flags round 1 for rollback review
    let r1_after: RoundStatus =
        http.get(api(&format!("/rounds/{}", r1.id))).bearer_auth(&trainer).send().await.unwrap().json().await.unwrap();
    assert_eq!(r1_after.units.get("accepted"), Some(&9));

    let inbox: Vec<InboxItem> =
        http.get(api("/admin/inbox?status=open")).bearer_auth(&admin).send().await.unwrap().json().await.unwrap();
    let mismatch_item = inbox.iter().find(|i| i.kind == "rerun-mismatch").expect("rerun-mismatch inbox item");
    assert!(mismatch_item.detail.contains("already-trained rounds affected"), "{}", mismatch_item.detail);

    // Sleeper's last 30 days of credit are frozen, and trust was demoted to probation
    let workers: Vec<WorkerInfo> =
        http.get(api("/admin/workers")).bearer_auth(&admin).send().await.unwrap().json().await.unwrap();
    let sw = workers.iter().find(|w| w.id == sleeper.id).unwrap();
    assert_eq!(sw.trust, TrustLevel::Probation);
    assert!(sw.credit_frozen > 0.0);
    assert_eq!(sw.credit_pending, 0.0);

    // Admin resolves inbox item as confirm-cheat -> bans sleeper and revokes frozen credits
    assert_eq!(
        http.post(api(&format!("/admin/inbox/{}/resolve", mismatch_item.id)))
            .bearer_auth(&admin)
            .json(&ResolveInbox { action: "confirm-cheat".into(), reason: "confirmed fake search".into() })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    // Banned/revoked token is now rejected
    assert_eq!(
        http.post(api("/lease"))
            .bearer_auth(&sleeper.token)
            .json(&LeaseRequest { host: sleeper_h.host })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn s1_database_migrates_cleanly_to_s2() {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    let tmp = std::env::temp_dir().join(format!("mana-s1-mig-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let db_path = tmp.join("swarm.db");

    // Create a pure S1-era SQLite database (no S2 columns)
    let opts =
        SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path.display())).unwrap().create_if_missing(true);
    let db = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    for stmt in [
        "CREATE TABLE tokens (id INTEGER PRIMARY KEY, hash TEXT NOT NULL UNIQUE, role TEXT NOT NULL, \
         name TEXT NOT NULL, created INTEGER NOT NULL, revoked INTEGER NOT NULL DEFAULT 0)",
        "CREATE TABLE hosts (id INTEGER PRIMARY KEY, token INTEGER NOT NULL, name TEXT NOT NULL, \
         build TEXT NOT NULL, class TEXT NOT NULL, strict INTEGER NOT NULL, threads INTEGER NOT NULL, os TEXT NOT NULL, \
         seen INTEGER NOT NULL, UNIQUE(token, name))",
        "CREATE TABLE rounds (id INTEGER PRIMARY KEY, project TEXT NOT NULL, run TEXT NOT NULL, \
         round INTEGER NOT NULL, priority INTEGER NOT NULL, created INTEGER NOT NULL, UNIQUE(project, run, round))",
        "CREATE TABLE units (id TEXT PRIMARY KEY, round INTEGER NOT NULL, unit TEXT NOT NULL, \
         state TEXT NOT NULL, host INTEGER, lease_until INTEGER, attempts INTEGER NOT NULL DEFAULT 0, manifest TEXT, \
         games INTEGER NOT NULL DEFAULT 0, done_at INTEGER, verdict TEXT)",
        "CREATE INDEX units_state ON units(state, round)",
        "CREATE TABLE uploads (unit TEXT NOT NULL, path TEXT NOT NULL, size INTEGER NOT NULL, \
         sha256 TEXT NOT NULL, PRIMARY KEY(unit, path))",
        "CREATE TABLE failures (unit TEXT NOT NULL, host INTEGER NOT NULL, PRIMARY KEY(unit, host))",
        "CREATE TABLE audit (id INTEGER PRIMARY KEY, at INTEGER NOT NULL, actor TEXT NOT NULL, \
         action TEXT NOT NULL, detail TEXT NOT NULL)",
        "INSERT INTO tokens (hash, role, name, created) VALUES ('deadbeef', 'trainer', 'trainer', 1000)",
    ] {
        sqlx::query(stmt).execute(&db).await.unwrap();
    }
    db.close().await;

    // Opening with S2 Swarm::open must migrate in place without errors (including units_source index)
    let store = Store::Local(Local::new(tmp.join("store"), "http://127.0.0.1:0".into()));
    let s = Swarm::open(db_path.to_str().unwrap(), store, "http://127.0.0.1:0").await.unwrap();
    let trust: String =
        sqlx::query_scalar("SELECT trust FROM tokens WHERE name = 'trainer'").fetch_one(&s.db).await.unwrap();
    assert_eq!(trust, "trusted");
    let _ = std::fs::remove_dir_all(&tmp);
}
