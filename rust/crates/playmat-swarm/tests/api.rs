//! The coordinator's lease rules over HTTP: a round's network links point at the coordinator whatever the trainer
//! wrote, and a host that gives a unit back never gets it again (another host does).

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
