//! The coordinator's lease rules over HTTP: a round's network links point at the coordinator whatever the trainer
//! wrote, a host that fails a unit never gets it again (another host does), and a released unit isn't a failure.

use playmat_swarm::store::{Local, Store};
use playmat_swarm::Swarm;
use playmat_swarm_proto::api::*;
use playmat_swarm_proto::{sha256_hex, NetRef, NetRole, Opponent, Pins, UnitKind, WorkUnit, PROTOCOL};
use reqwest::StatusCode;

fn unit(net: &str) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: "t-r1-u000".into(),
        project: "test".into(),
        run: "api".into(),
        round: 1,
        kind: UnitKind::Selfplay,
        pins: Pins {
            builds: vec![],
            obs: 3,
            suite: "constructed-2".into(),
            suite_sha256: "0".repeat(64),
            strict: true,
        },
        nets: vec![NetRef { role: NetRole::Learner, sha256: net.into(), url: "http://127.0.0.1:1/elsewhere".into() }],
        learner: format!("cfg:policy=ismcts+net=@{net}+iters=4"),
        opponents: vec![Opponent { config: "host@s6".into(), weight: 1 }],
        seed: 1,
        games: 2,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 1.0,
    }
}

#[tokio::test]
async fn leases_follow_the_rules() {
    let tmp = std::env::temp_dir().join(format!("mana-api-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let worker = s.new_token(Role::Worker, "alice").await.unwrap().token;
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    // a network, uploaded the trainer's way
    let net = b"not really a network".to_vec();
    let sha = sha256_hex(&net);
    let urls: Urls = http
        .post(api(&format!("/nets/{sha}?size={}", net.len())))
        .bearer_auth(&trainer)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let put = http.put(&urls.urls[&format!("{sha}.pmnet")]).body(net.clone()).send().await.unwrap();
    assert!(put.status().is_success());

    let round = NewRound { project: "test".into(), run: "api".into(), round: 1, priority: 0, units: vec![unit(&sha)] };
    let res = http.post(api("/rounds")).bearer_auth(&trainer).json(&round).send().await.unwrap();
    assert!(res.status().is_success(), "{}", res.text().await.unwrap());

    let hello = |name: &str| Hello {
        name: name.into(),
        build: "dev".into(),
        class: "strict-1".into(),
        strict: true,
        threads: 1,
        os: "test".into(),
    };
    let laptop: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&worker)
        .json(&hello("laptop"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let desktop: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&worker)
        .json(&hello("desktop"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let lease = |host: i64| http.post(api("/lease")).bearer_auth(&worker).json(&LeaseRequest { host }).send();

    let l: Lease = lease(laptop.host).await.unwrap().json().await.unwrap();
    assert_eq!(l.unit.nets[0].url, format!("{base}/api/v1/nets/{sha}"));
    let got = http.get(&l.unit.nets[0].url).bearer_auth(&worker).send().await.unwrap().bytes().await.unwrap();
    assert_eq!(got.to_vec(), net);

    // a release hands it back with no mark against the host: it may lease it again
    let res = http.post(api("/units/t-r1-u000/release")).bearer_auth(&worker).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let l: Lease = lease(laptop.host).await.unwrap().json().await.unwrap();
    assert_eq!(l.unit.id, "t-r1-u000");

    let res = http
        .post(api("/units/t-r1-u000/fail"))
        .bearer_auth(&worker)
        .json(&Fail { reason: "test".into() })
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(lease(laptop.host).await.unwrap().status(), StatusCode::NO_CONTENT);
    let l: Lease = lease(desktop.host).await.unwrap().json().await.unwrap();
    assert_eq!(l.unit.id, "t-r1-u000");
    let _ = std::fs::remove_dir_all(&tmp);
}

fn plain_unit(id: &str, round: u32, seed: u64) -> WorkUnit {
    WorkUnit {
        protocol: PROTOCOL,
        id: id.into(),
        project: "test".into(),
        run: "admin".into(),
        round,
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
        games: 2,
        keep: 200,
        fast: None,
        deadline: 0,
        cost: 1.0,
    }
}

#[tokio::test]
async fn rounds_can_be_reprioritized_and_units_cancelled() {
    let tmp = std::env::temp_dir().join(format!("mana-admin-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let store = Store::Local(Local::new(tmp.join("store"), base.clone()));
    let s = Swarm::open(tmp.join("swarm.db").to_str().unwrap(), store, &base).await.unwrap();
    let worker = s.new_token(Role::Worker, "alice").await.unwrap().token;
    let trainer = s.new_token(Role::Trainer, "trainer").await.unwrap().token;
    let admin = s.new_token(Role::Admin, "owner").await.unwrap().token;
    tokio::spawn(playmat_swarm::serve(listener, s));
    let http = reqwest::Client::new();
    let api = |p: &str| format!("{base}/api/v1{p}");

    let r1: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound {
            project: "test".into(),
            run: "admin".into(),
            round: 1,
            priority: 0,
            units: vec![plain_unit("r1-u000", 1, 1), plain_unit("r1-u001", 1, 3), plain_unit("r1-u002", 1, 5)],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let r2: RoundStatus = http
        .post(api("/rounds"))
        .bearer_auth(&trainer)
        .json(&NewRound {
            project: "test".into(),
            run: "admin".into(),
            round: 2,
            priority: 0,
            units: vec![plain_unit("r2-u000", 2, 10)],
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // worker can't reprioritize or cancel
    assert_eq!(
        http.post(api(&format!("/rounds/{}/priority", r2.id)))
            .bearer_auth(&worker)
            .json(&RoundPriority { priority: 10 })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        http.post(api("/units/r1-u000/cancel"))
            .bearer_auth(&worker)
            .json(&Cancel::default())
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    // raise round 2's priority so it jumps ahead of round 1
    let bumped: RoundStatus = http
        .post(api(&format!("/rounds/{}/priority", r2.id)))
        .bearer_auth(&admin)
        .json(&RoundPriority { priority: 10 })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(bumped.priority, 10);

    let rounds: Vec<RoundStatus> =
        http.get(api("/rounds")).bearer_auth(&trainer).send().await.unwrap().json().await.unwrap();
    assert_eq!(rounds.len(), 2);
    assert_eq!((rounds[0].id, rounds[0].priority), (r2.id, 10));

    let host: HelloReply = http
        .post(api("/hello"))
        .bearer_auth(&worker)
        .json(&Hello {
            name: "box".into(),
            build: "dev".into(),
            class: "strict-1".into(),
            strict: true,
            threads: 1,
            os: "test".into(),
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let lease = || http.post(api("/lease")).bearer_auth(&worker).json(&LeaseRequest { host: host.host }).send();

    // round 2 goes first
    let l: Lease = lease().await.unwrap().json().await.unwrap();
    assert_eq!(l.unit.id, "r2-u000");

    // cancelling a leased unit immediately invalidates its heartbeat
    assert_eq!(
        http.post(api("/units/r2-u000/cancel"))
            .bearer_auth(&admin)
            .json(&Cancel { reason: "superseded".into(), include_leased: false })
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        http.post(api("/units/r2-u000/heartbeat")).bearer_auth(&worker).send().await.unwrap().status(),
        StatusCode::CONFLICT
    );
    // cancelling an already-cancelled unit is a conflict
    assert_eq!(
        http.post(api("/units/r2-u000/cancel"))
            .bearer_auth(&admin)
            .json(&Cancel::default())
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );

    // lease one unit of round 1, then cancel round 1's remaining queued units without touching the leased one
    let l: Lease = lease().await.unwrap().json().await.unwrap();
    assert_eq!(l.unit.id, "r1-u000");
    let reply: CancelReply = http
        .post(api(&format!("/rounds/{}/cancel", r1.id)))
        .bearer_auth(&trainer)
        .json(&Cancel { reason: "cutoff".into(), include_leased: false })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reply.cancelled, 2);
    assert_eq!(reply.round.units.get("leased"), Some(&1));
    assert_eq!(reply.round.units.get("cancelled"), Some(&2));
    // heartbeat on the leased unit still succeeds
    assert_eq!(
        http.post(api("/units/r1-u000/heartbeat")).bearer_auth(&worker).send().await.unwrap().status(),
        StatusCode::NO_CONTENT
    );

    // cancelling with include_leased=true stops the in-flight lease too
    let reply: CancelReply = http
        .post(api(&format!("/rounds/{}/cancel", r1.id)))
        .bearer_auth(&admin)
        .json(&Cancel { reason: "stop all".into(), include_leased: true })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reply.cancelled, 1);
    assert_eq!(reply.round.units.get("cancelled"), Some(&3));
    assert_eq!(lease().await.unwrap().status(), StatusCode::NO_CONTENT);
    let _ = std::fs::remove_dir_all(&tmp);
}
