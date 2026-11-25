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
        jobs: 1,
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
    let dir = tmp.join("check");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("unit.json"), serde_json::to_string(&r.work).unwrap()).unwrap();
    std::fs::write(dir.join("manifest.json"), serde_json::to_string(&r.manifest).unwrap()).unwrap();
    for (path, url) in &r.files {
        std::fs::write(dir.join(path), http.get(url).send().unwrap().bytes().unwrap()).unwrap();
    }
    verify(dir.to_str().unwrap(), &repo).unwrap();

    let v = Verdict { accept: true, reason: "replayed".into() };
    let res = http.post(api(&format!("/units/{}/verdict", r.unit))).bearer_auth(&trainer).json(&v).send().unwrap();
    assert!(res.status().is_success());
    let st: Status = http.get(api("/status")).send().unwrap().json().unwrap();
    assert_eq!((st.games_accepted, st.hosts, st.units.get("accepted").copied()), (2, 1, Some(1)));
    let _ = std::fs::remove_dir_all(&tmp);
}
