//! Self-play's pieces (#390 Track B): the observation shows nothing hidden, the Rust network matches PyTorch's, and
//! network policies play whole games.

use playmat_ai::decision::{Hidden, PolicyKind, Root};
use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::net::{self, Net};
use playmat_ai::obs::{self, Obs};
use playmat_ai::policy::{Policy, Repeats};
use playmat_ai::search::{candidates, Config, Table};
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::rules;
use playmat_engine::state::{Game, Seat};

fn deck(name: &str) -> Deck {
    let path = format!("{}/../../../Mage.Server/career-src/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    Deck::parse(name, &std::fs::read_to_string(path).unwrap())
}

/// Suite decks with instants, counters, discard and graveyard play
const PAIRS: [(&str, &str); 3] = [
    ("pioneer/act2-dimir-control", "pioneer/act3-izzet-phoenix"),
    ("pioneer/act1-mono-black-devotion", "pioneer/act1-boros-aggro"),
    ("standard/meta-jund-sacrifice", "standard/meta-azorius-control"),
];

fn decks() -> (Deck, Deck) {
    (deck(PAIRS[0].0), deck(PAIRS[0].1))
}

/// Plays a game with the quick answers, and at each of seat 0's questions checks that the observation of every
/// resample of what seat 0 can't see is the observation of the real game
struct Audit {
    policy: [Policy; 2],
    repeats: Repeats,
    hidden: Hidden,
    checked: usize,
    differed: usize,
    answers: usize,
}

impl Decider for Audit {
    fn answer(&mut self, game: &Game, seat: Seat, q: &Query) -> Answer {
        let quick = match q {
            Query::Priority { playable } => self.policy[seat].priority(game, seat, playable),
            _ => self.policy[seat].answer(game, seat, q),
        };
        self.hidden.observe(game);
        self.answers += 1;
        // as the table does (`max_answers`): a game stuck re-asking the same question stops
        if self.answers > 3000 {
            return Answer::Stop;
        }
        if seat == 0 && !matches!(q, Query::PlayMana { .. }) {
            let cands = candidates(game, seat, q, &quick, 12);
            let real = obs::observe(game, seat, q, &cands);
            // the cards the question shows the seat are seen: the sample keeps the objects answers name (`since`)
            let shown: Vec<Answer> = cands.iter().filter(|a| matches!(a, Answer::Obj(_))).cloned().collect();
            let root = Root { snapshot: game.clone(), since: shown, hidden: Some(self.hidden.clone()), playout: 2 };
            for s in 0..3 {
                let g = root.sample(self.checked as u64 * 7 + s);
                // the candidates name the same objects: only those in what the seat can see may differ
                let o = obs::observe(&g, seat, q, &cands);
                self.checked += 1;
                if !same(&real, &o) {
                    self.differed += 1;
                    let d = |x: &[f32], y: &[f32]| x.iter().zip(y).position(|(a, b)| a != b);
                    eprintln!(
                        "leak at {q:?}: glob {:?} ents {:?} acts {:?} ids {} {}",
                        d(&real.glob, &o.glob),
                        d(&real.ents, &o.ents).map(|i| i % obs::ENT_F),
                        d(&real.acts, &o.acts).map(|i| (i / obs::ACT_F, i % obs::ACT_F)),
                        real.ent_ids == o.ent_ids,
                        real.act_ids == o.act_ids
                    );
                }
            }
        }
        if let Query::Priority { .. } = q {
            self.policy[seat].played(&quick);
        }
        let options = candidates(game, seat, q, &quick, usize::MAX);
        self.repeats.guard(seat, q, quick, &options)
    }

    fn stopped(&self) -> bool {
        self.answers > 3000
    }
}

fn same(a: &Obs, b: &Obs) -> bool {
    a.glob == b.glob && a.ents == b.ents && a.ent_ids == b.ent_ids && a.acts == b.acts && a.act_ids == b.act_ids
}

#[test]
fn the_observation_shows_nothing_the_seat_cant_see() {
    let db = playmat_engine::cards();
    let mut total = 0;
    for seed in 0..9 {
        let (a, b) = (deck(PAIRS[seed as usize % 3].0), deck(PAIRS[seed as usize % 3].1));
        let mut game = new_duel(&db, [&a, &b], seed, (seed % 2) as usize).unwrap();
        let mut audit = Audit {
            policy: [Policy::version(2), Policy::version(2)],
            repeats: Repeats::default(),
            hidden: Hidden::new(0),
            checked: 0,
            differed: 0,
            answers: 0,
        };
        rules::play_from_priority(&mut game, &mut audit, 12);
        assert_eq!(
            audit.differed, 0,
            "seed {seed}: {} of {} resamples changed the observation",
            audit.differed, audit.checked
        );
        total += audit.checked;
    }
    assert!(total > 300, "only {total} observations checked");
}

#[test]
fn the_opponents_hand_changes_the_critics_view_only() {
    let db = playmat_engine::cards();
    let (a, b) = decks();
    let game = new_duel(&db, [&a, &b], 3, 0).unwrap();
    let q = Query::Priority { playable: Vec::new() };
    let mut h = Hidden::new(0);
    h.observe(&game);
    let root = Root { snapshot: game.clone(), since: Vec::new(), hidden: Some(h), playout: 2 };
    let g = (0..20).map(|s| root.sample(s)).find(|g| g.player(1).hand != game.player(1).hand).unwrap();
    assert!(same(&obs::observe(&game, 0, &q, &[]), &obs::observe(&g, 0, &q, &[])));
    assert_ne!(obs::hidden_entities(&game, 0).1, obs::hidden_entities(&g, 0).1);
}

/// `rust/rl/parity.py` writes observations and what PyTorch's network made of them, for the shipped network
#[test]
fn the_rust_network_matches_pytorch() {
    let net = net::load("builtin").unwrap();
    let cases = parity(&net, "net_parity.json");
    assert_eq!(
        cases["sha"].as_str().unwrap(),
        sha(net::BUILTIN),
        "net_parity.json is for another net.pmnet: rerun parity.py"
    );
}

/// The transformer (#400 R2), on a small random one
#[test]
fn the_rust_transformer_matches_pytorch() {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/tx_small.pmnet")).unwrap();
    let net = Net::parse("tx_small", &bytes).unwrap();
    assert!(matches!(net, Net::Tx(_)));
    let cases = parity(&net, "net_parity_tx.json");
    assert_eq!(cases["sha"].as_str().unwrap(), sha(&bytes));
}

/// Checks `net` against a fixture `parity.py` wrote, and gives the fixture
fn parity(net: &Net, fixture: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(format!("{}/tests/{fixture}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let cases: serde_json::Value = serde_json::from_str(&text).unwrap();
    let f = |v: &serde_json::Value| -> Vec<f32> {
        v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect()
    };
    let u = |v: &serde_json::Value| -> Vec<u32> {
        v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u32).collect()
    };
    let mut n = 0;
    for c in cases["cases"].as_array().unwrap() {
        let o = Obs {
            glob: f(&c["glob"]),
            ents: f(&c["ents"]),
            ent_ids: u(&c["ent_ids"]),
            acts: f(&c["acts"]),
            act_ids: u(&c["act_ids"]),
        };
        let out = net.forward(&o);
        let want = f(&c["logits"]);
        for (x, y) in out.logits.iter().zip(&want) {
            assert!((x - y).abs() < 2e-3, "logit {x} against PyTorch's {y}");
        }
        assert!((out.value - c["value"].as_f64().unwrap() as f32).abs() < 2e-3);
        let bel = net.belief(&out.h, &u(&c["pool"]));
        for (x, y) in bel.iter().zip(&f(&c["belief"])) {
            assert!((x - y).abs() < 2e-3, "belief {x} against PyTorch's {y}");
        }
        n += 1;
    }
    assert!(n >= 4);
    cases
}

fn sha(b: &[u8]) -> String {
    format!("{:016x}", b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, x| (h ^ *x as u64).wrapping_mul(0x0100_0000_01b3)))
}

#[test]
fn network_policies_play_whole_games() {
    let db = playmat_engine::cards();
    let (a, b) = decks();
    for (kind, hide) in [(PolicyKind::Net, false), (PolicyKind::NetSearch, true)] {
        let cfg =
            Config { policy: kind, hide, samples: 2, budget: 1500, eval: 3, temp: 0, threads: 1, ..Config::default() };
        let mut game = new_duel(&db, [&a, &b], 11, 0).unwrap();
        let mut table = Table::new(vec![Some(cfg), None]);
        rules::play_from_priority(&mut game, &mut table, 30);
        assert!(!table.stopped_early());
        assert!(table.stats.len() > 5, "{kind:?} answered {} questions", table.stats.len());
        assert!(game.player(0).lost || game.player(1).lost || game.turn >= 30);
    }
}

#[test]
fn half_floats_round_trip() {
    for v in [0.0f32, 1.0, -2.5, 0.2, 1.0 / 3.0, 65504.0, 1e-5] {
        let h = playmat_ai::selfplay::f16(v);
        // decode
        let s = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
        let e = ((h >> 10) & 0x1f) as i32;
        let m = (h & 0x3ff) as f32;
        let d = if e == 0 { s * m * 2f32.powi(-24) } else { s * (1.0 + m / 1024.0) * 2f32.powi(e - 15) };
        assert!((d - v).abs() <= v.abs() * 1e-3 + 1e-6, "{v} -> {d}");
    }
}

/// Time per forward pass of the network at `PLAYMAT_NET_BENCH` on a 30-entity, 10-candidate observation:
/// `PLAYMAT_NET_BENCH=path cargo test --release --test self_play net_speed -- --ignored --nocapture`
#[test]
#[ignore]
fn net_speed() {
    let path = std::env::var("PLAYMAT_NET_BENCH").unwrap_or_else(|_| "builtin".into());
    let net = net::load(&path).unwrap();
    let o = Obs {
        glob: vec![0.3; obs::GLOB_F],
        ents: (0..30 * obs::ENT_F).map(|i| (i % 7) as f32 / 7.0).collect(),
        ent_ids: (0..30).map(|i| i * 37 % 4000 + 1).collect(),
        acts: (0..10 * obs::ACT_F).map(|i| (i % 5) as f32 / 5.0).collect(),
        act_ids: (0..10).map(|i| i * 91 % 4000 + 1).collect(),
    };
    let start = std::time::Instant::now();
    let n = 200;
    let mut s = 0.0;
    for _ in 0..n {
        s += net.forward(&o).value;
    }
    println!("{path}: {:.3} ms a forward pass ({s:.1})", start.elapsed().as_secs_f64() * 1000.0 / n as f64);
}
