//! Fits the fitted score (`Config::eval` 2, `fitted.rs`): plays self-play games, records each turn's position at the
//! start of its first main phase, and fits a logistic regression (L2) of the game's result on the position's features
//! and card counts. Writes the weights as `fitted.json`.
//!
//! cargo run --release -p playmat-ai --bin fit-eval -- [--entrant cfg:policy=quick+playout=2] [--games 6888]
//!     [--suite archbench/constructed-2] [--jobs 10] [--l2 0.001] [--min-card 100] [--out crates/playmat-ai/src/fitted.json]
//!
//! A tenth of the games (every tenth deal) is held out: the output compares the fit's log loss on them with Java's
//! score squashed into a chance, so a refit can be checked before it ships.

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::entrants::entrant;
use playmat_ai::fitted::{self, Weights, NAMES};
use playmat_ai::search::Table;
use playmat_ai::{eval, mulligan};
use playmat_engine::decide::{Answer, Decider, Query};
use playmat_engine::rules;
use playmat_engine::state::{Game, Seat, Step};
use serde_json::Value;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// A position from seat 0's side: features, card counts and Java's score
struct Position {
    deal: usize,
    decks: [usize; 2],
    f: [f64; 17],
    cards: Vec<(String, f64)>,
    java: i64,
}

/// The table, recording a position once a turn
struct Recorder {
    table: Table,
    turn: u32,
    positions: Vec<Position>,
}

impl Decider for Recorder {
    fn answer(&mut self, game: &Game, seat: Seat, query: &Query) -> Answer {
        if matches!(query, Query::Priority { .. })
            && game.step == Some(Step::PrecombatMain)
            && game.turn != self.turn
            && game.stack.is_empty()
        {
            self.turn = game.turn;
            self.positions.push(Position {
                deal: 0,
                decks: [0, 0],
                f: fitted::features(game, 0),
                cards: fitted::card_counts(game, 0),
                java: eval::evaluate(game, 0),
            });
        }
        self.table.answer(game, seat, query)
    }
    fn traced(&mut self, game: &Game) {
        self.table.traced(game)
    }
    fn stopped(&self) -> bool {
        self.table.stopped()
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let repo = env!("CARGO_MANIFEST_DIR").to_string() + "/../../..";
    let suite_dir = opt("--suite").unwrap_or_else(|| format!("{repo}/archbench/constructed-2"));
    let suite: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{suite_dir}/suite.json")).unwrap()).unwrap();
    let decks: Vec<Deck> = suite["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let path = d["file"].as_str().unwrap();
            let text = std::fs::read_to_string(format!("{repo}/{path}")).unwrap();
            Deck::parse(std::path::Path::new(path).file_stem().unwrap().to_str().unwrap(), &text)
        })
        .collect();
    let n = decks.len();
    let who = entrant(&opt("--entrant").unwrap_or_else(|| "cfg:policy=quick+playout=2".into())).unwrap();
    let games: usize = opt("--games").and_then(|v| v.parse().ok()).unwrap_or(n * (n - 1) * 4);
    let jobs: usize = opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or(10);
    let l2: f64 = opt("--l2").and_then(|v| v.parse().ok()).unwrap_or(0.001);
    let min_card: usize = opt("--min-card").and_then(|v| v.parse().ok()).unwrap_or(100);
    let out = opt("--out").unwrap_or_else(|| format!("{}/src/fitted.json", env!("CARGO_MANIFEST_DIR")));
    let turns = suite["turn_cap"].as_u64().unwrap_or(30) as u32;
    let mulligans = suite["mulligans"].as_bool().unwrap_or(false);
    let db = playmat_engine::cards();

    // play
    let started = std::time::Instant::now();
    let next = AtomicUsize::new(0);
    let data: Mutex<Vec<(Position, f64)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| loop {
                let deal = next.fetch_add(1, Ordering::Relaxed);
                if deal >= games {
                    break;
                }
                let a = deal % n;
                let b = (a + 1 + (deal / n) % (n - 1)) % n;
                let Ok(mut game) = new_duel(&db, [&decks[a], &decks[b]], deal as i64 + 1_000_000, deal % 2) else {
                    continue;
                };
                let config = who.config.clone();
                let run = catch_unwind(AssertUnwindSafe(|| {
                    if mulligans {
                        mulligan::london(
                            &mut game,
                            deal % 2,
                            0,
                            &mut |g, seat, taken| mulligan::wants(g, seat, taken, config.as_ref()),
                            &mut |g, seat| mulligan::bottom(g, seat, config.as_ref()),
                        );
                    }
                    let mut rec = Recorder {
                        table: Table::new(vec![config.clone(), config.clone()]),
                        turn: 0,
                        positions: vec![],
                    };
                    rules::play_from_priority(&mut game, &mut rec, turns);
                    rec
                }));
                let Ok(rec) = run else { continue };
                if rec.table.stopped_early() {
                    continue;
                }
                let result = if game.player(0).lost {
                    0.0
                } else if game.player(1).lost {
                    1.0
                } else {
                    0.5
                };
                let mut d = data.lock().unwrap();
                for mut p in rec.positions {
                    p.deal = deal;
                    p.decks = [a, b];
                    d.push((p, result));
                }
                if deal.is_multiple_of(500) {
                    eprintln!(
                        "FIT played {deal}/{games}, {} positions, {:.0} s",
                        d.len(),
                        started.elapsed().as_secs_f64()
                    );
                }
            });
        }
    });
    let data = data.into_inner().unwrap();
    let (train, test): (Vec<_>, Vec<_>) = data.iter().partition(|(p, _)| p.deal % 10 != 0);
    println!(
        "FIT {} positions ({} held out) from {games} games, {:.0} s",
        data.len(),
        test.len(),
        started.elapsed().as_secs_f64()
    );

    // card names seen often enough to get a weight of their own
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (p, _) in &train {
        for (c, _) in &p.cards {
            *seen.entry(c.as_str()).or_default() += 1;
        }
    }
    let mut names: Vec<&str> = seen.iter().filter(|(_, n)| **n >= min_card).map(|(c, _)| *c).collect();
    names.sort();
    let index: HashMap<&str, usize> = names.iter().enumerate().map(|(i, c)| (*c, NAMES.len() + i)).collect();
    // each deck's strength, a nuisance term: without it card weights learn which deck is playing, not what a card on
    // the battlefield is worth. Dropped from the weights that ship.
    let deck_base = NAMES.len() + names.len();
    let dim = deck_base + n;
    let rows = |set: &[&(Position, f64)]| -> Vec<(Vec<(usize, f64)>, f64)> {
        set.iter()
            .map(|(p, y)| {
                let mut x: Vec<(usize, f64)> = p.f.iter().copied().enumerate().collect();
                for (c, v) in &p.cards {
                    if let Some(i) = index.get(c.as_str()) {
                        x.push((*i, *v));
                    }
                }
                if p.decks[0] != p.decks[1] {
                    x.push((deck_base + p.decks[0], 1.0));
                    x.push((deck_base + p.decks[1], -1.0));
                }
                (x, *y)
            })
            .collect()
    };
    let (tr, te) = (rows(&train), rows(&test));

    // fit: full-batch Adam on the mean log loss plus L2 on every weight (no bias: the model is antisymmetric)
    let mut w = vec![0.0f64; dim];
    let (mut m, mut v) = (vec![0.0f64; dim], vec![0.0f64; dim]);
    let (lr, b1, b2) = (0.05, 0.9, 0.999);
    let sig = |z: f64| 1.0 / (1.0 + (-z).exp());
    for step in 1..=600 {
        let mut g = vec![0.0f64; dim];
        for (x, y) in &tr {
            let z: f64 = x.iter().map(|(i, xi)| w[*i] * xi).sum();
            let e = sig(z) - y;
            for (i, xi) in x {
                g[*i] += e * xi;
            }
        }
        for i in 0..dim {
            g[i] = g[i] / tr.len() as f64 + l2 * w[i];
            m[i] = b1 * m[i] + (1.0 - b1) * g[i];
            v[i] = b2 * v[i] + (1.0 - b2) * g[i] * g[i];
            let (mh, vh) = (m[i] / (1.0 - b1.powi(step)), v[i] / (1.0 - b2.powi(step)));
            w[i] -= lr * mh / (vh.sqrt() + 1e-8);
        }
    }
    let loss = |set: &[(Vec<(usize, f64)>, f64)], w: &[f64]| {
        set.iter()
            .map(|(x, y)| {
                let p = sig(x.iter().map(|(i, xi)| w[*i] * xi).sum::<f64>()).clamp(1e-6, 1.0 - 1e-6);
                -(y * p.ln() + (1.0 - y) * (1.0 - p).ln())
            })
            .sum::<f64>()
            / set.len().max(1) as f64
    };
    // Java's score as a chance, at its best scale on the training set
    let java_loss = |set: &[&(Position, f64)], k: f64| {
        set.iter()
            .map(|(p, y)| {
                let p = sig(p.java.clamp(-1_000_000, 1_000_000) as f64 / k).clamp(1e-6, 1.0 - 1e-6);
                -(y * p.ln() + (1.0 - y) * (1.0 - p).ln())
            })
            .sum::<f64>()
            / set.len().max(1) as f64
    };
    let k = [500.0, 1000.0, 2000.0, 3000.0, 5000.0, 8000.0, 12000.0]
        .into_iter()
        .min_by(|a, b| java_loss(&train, *a).total_cmp(&java_loss(&train, *b)))
        .unwrap();
    let generic: Vec<f64> = w[..NAMES.len()].to_vec();
    let generic_only: Vec<f64> = generic.iter().copied().chain(std::iter::repeat_n(0.0, dim - NAMES.len())).collect();
    // what ships: no deck terms
    let shipped: Vec<f64> = w[..deck_base].iter().copied().chain(std::iter::repeat_n(0.0, n)).collect();
    println!(
        "FIT log loss held out: fitted {:.4} (with deck terms {:.4}, generic features alone {:.4}), Java's score {:.4} (scale {k}), coin 0.6931",
        loss(&te, &shipped),
        loss(&te, &w),
        loss(&te, &generic_only),
        java_loss(&test, k)
    );
    for (name, wi) in NAMES.iter().zip(&generic) {
        println!("FIT {name:18} {wi:+.3}");
    }
    let mut cards: Vec<(&str, f64)> = names.iter().map(|c| (*c, w[index[c]])).collect();
    cards.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("FIT {} card weights; top {:?}", cards.len(), &cards[..cards.len().min(8)]);
    println!("FIT bottom {:?}", &cards[cards.len().saturating_sub(8)..]);
    let weights = Weights {
        features: generic,
        cards: cards.iter().map(|(c, w)| (c.to_string(), (w * 1000.0).round() / 1000.0)).collect(),
    };
    std::fs::write(&out, serde_json::to_string_pretty(&weights).unwrap() + "\n").unwrap();
    println!("FIT wrote {out}");
}
