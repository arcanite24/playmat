//! Plays and records games for training the network (#390 Track B; `rust/rl` reads what it writes).
//!
//! cargo run --release -p playmat-ai --bin selfplay -- --out DIR [--learner ENTRANT] [--opponents "self*7;host@s2*1"]
//!     [--games 2000] [--jobs 10] [--keep 1000] [--fast net] [--seed 0] [--tag 0] [--suite archbench/constructed-2]
//!     [--repo DIR]
//!
//! Each game, one seat is the learner and the other an opponent drawn from `--opponents` (`name*weight` joined by
//! `;`; `self` is the learner again, and then both seats are recorded). Deck pairs go round the suite's ordered
//! pairs, the seats and who plays first alternate, and mulligans follow the suite. `--keep` is the chance per
//! thousand that a decision is recorded; with `--fast KIND` (`net`, `quick`, any policy), the others are answered by
//! that policy instead of the learner's, so the search is spent only on recorded decisions. One shard of files per
//! job goes in `--out` (`selfplay::Shard`).
//!
//! Learners: any Archbench entrant (`host@s6` records the search AI, for imitation), or the network:
//! `cfg:policy=net+net=PATH+temp=1000` samples its policy, `cfg:policy=netsearch+hide=1+samples=4+net=PATH` searches.

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::entrants::{entrant, Entrant};
use playmat_ai::search::Table;
use playmat_ai::selfplay::{mix, Recording, Shard, Sink};
use playmat_ai::{eval, mulligan};
use playmat_engine::rules;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    // deck files are named from the repository's root; `--repo` moves it (the image has only the suite's decks)
    let repo = opt("--repo").unwrap_or_else(|| env!("CARGO_MANIFEST_DIR").to_string() + "/../../..");
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
    let learner = entrant(&opt("--learner").unwrap_or_else(|| "host@s6".into())).unwrap_or_else(|e| panic!("{e}"));
    let mut opponents: Vec<(Option<Entrant>, u64)> = Vec::new();
    for part in opt("--opponents").unwrap_or_else(|| "self".into()).split(';').filter(|s| !s.is_empty()) {
        let (name, w) = part.rsplit_once('*').map_or((part, 1), |(a, b)| (a, b.parse().unwrap_or(1)));
        let e = (name != "self").then(|| entrant(name).unwrap_or_else(|e| panic!("{e}")));
        opponents.push((e, w));
    }
    let total_w: u64 = opponents.iter().map(|o| o.1).sum();
    let games: usize = opt("--games").and_then(|v| v.parse().ok()).unwrap_or(2000);
    let jobs: usize = opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or(10);
    let keep: u32 = opt("--keep").and_then(|v| v.parse().ok()).unwrap_or(1000);
    let (fast, fast_iters): (Option<playmat_ai::decision::PolicyKind>, Option<u32>) = match opt("--fast") {
        None => (None, None),
        Some(v) if v.starts_with("ismcts:") => {
            let n: u32 = v[7..].parse().unwrap_or_else(|e| panic!("--fast {v}: {e}"));
            (None, Some(n))
        }
        Some(v) => (Some(v.parse().unwrap_or_else(|e| panic!("--fast {v}: {e}"))), None),
    };
    let seed: usize = opt("--seed").and_then(|v| v.parse().ok()).unwrap_or(0);
    let tag: i32 = opt("--tag").and_then(|v| v.parse().ok()).unwrap_or(0);
    let out = opt("--out").expect("--out DIR");
    let turns = suite["turn_cap"].as_u64().unwrap_or(30) as u32;
    let mulligans = suite["mulligans"].as_bool().unwrap_or(false);
    let db = playmat_engine::cards();

    let started = std::time::Instant::now();
    let next = AtomicUsize::new(0);
    // per opponent: learner wins, draws, losses
    let tally: Mutex<Vec<[u32; 3]>> = Mutex::new(vec![[0; 3]; opponents.len()]);
    let steps = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for job in 0..jobs {
            let (db, decks, learner, opponents, tally, steps, failed, next, out) =
                (&db, &decks, &learner, &opponents, &tally, &steps, &failed, &next, &out);
            s.spawn(move || {
                let mut shard = Shard::create(out, &format!("w{job:02}")).expect("shard");
                loop {
                    let g = next.fetch_add(1, Ordering::Relaxed);
                    if g >= games {
                        break;
                    }
                    let deal = seed + g;
                    let a = deal % n;
                    let b = (a + 1 + (deal / n) % (n - 1)) % n;
                    let mut r = (deal as u64) ^ 0xa076_1d64_78bd_642f;
                    let mut pick = mix(&mut r) % total_w;
                    let oi = opponents.iter().position(|(_, w)| {
                        if pick < *w {
                            true
                        } else {
                            pick -= w;
                            false
                        }
                    });
                    let oi = oi.unwrap_or(0);
                    let opp = opponents[oi].0.as_ref().unwrap_or(learner);
                    let me = (deal / 2) % 2;
                    let mut configs = [opp.config.clone(), opp.config.clone()];
                    configs[me] = learner.config.clone();
                    let record = [opponents[oi].0.is_none() || me == 0, opponents[oi].0.is_none() || me == 1];
                    let Ok(mut game) = new_duel(db, [&decks[a], &decks[b]], deal as i64 + 7_000_000, deal % 2) else {
                        continue;
                    };
                    let sink: Sink = Arc::new(Mutex::new(Vec::new()));
                    let run = catch_unwind(AssertUnwindSafe(|| {
                        if mulligans {
                            mulligan::london(
                                &mut game,
                                deal % 2,
                                0,
                                &mut |g, seat, taken| mulligan::wants(g, seat, taken, configs[seat].as_ref()),
                                &mut |g, seat| mulligan::bottom(g, seat, configs[seat].as_ref()),
                            );
                        }
                        let mut table = Table::new(configs.to_vec());
                        for seat in 0..2 {
                            if let (true, Some(c)) = (record[seat], configs[seat].as_ref()) {
                                let rec = Recording::new(
                                    c.policy,
                                    seat,
                                    Arc::clone(&sink),
                                    keep,
                                    (deal as u64) << 1 | seat as u64,
                                )
                                .with_fast(fast, seat)
                                .with_fast_iters(fast_iters);
                                table.set_policy(seat, Box::new(rec));
                            }
                        }
                        rules::play_from_priority(&mut game, &mut table, turns);
                        table.stopped_early()
                    }));
                    if !matches!(run, Ok(false)) {
                        failed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    let winner = if game.player(0).lost || eval::evaluate(&game, 1) == eval::WIN {
                        Some(1)
                    } else if game.player(1).lost || eval::evaluate(&game, 0) == eval::WIN {
                        Some(0)
                    } else {
                        None
                    };
                    let result = match winner {
                        Some(w) => [(w == 0) as u8 as f32, (w == 1) as u8 as f32],
                        None => [0.5, 0.5],
                    };
                    {
                        let mut t = tally.lock().unwrap();
                        let k = match winner {
                            Some(w) if w == me => 0,
                            None => 1,
                            _ => 2,
                        };
                        t[oi][k] += 1;
                    }
                    let recorded = std::mem::take(&mut *sink.lock().unwrap());
                    steps.fetch_add(recorded.len(), Ordering::Relaxed);
                    let info = json!({
                        "deal": deal, "decks": [decks[a].name, decks[b].name], "learner": me,
                        "opponent": opp.name, "turns": game.turn, "first": deal % 2,
                    });
                    shard.game(deal, tag, &recorded, result, info).expect("write");
                    if g % 500 == 0 {
                        use std::io::Write;
                        let _ = writeln!(
                            std::io::stderr(),
                            "SELFPLAY {g}/{games} games, {} steps, {:.0} s",
                            steps.load(Ordering::Relaxed),
                            started.elapsed().as_secs_f64()
                        );
                    }
                }
                shard.flush().expect("flush");
            });
        }
    });
    let secs = started.elapsed().as_secs_f64();
    println!(
        "SELFPLAY {games} games ({} failed), {} steps, {:.0} s, {:.1} games/s; learner {}",
        failed.load(Ordering::Relaxed),
        steps.load(Ordering::Relaxed),
        secs,
        games as f64 / secs,
        learner.name
    );
    for (i, t) in tally.into_inner().unwrap().iter().enumerate() {
        let name = opponents[i].0.as_ref().map_or("self", |e| e.name.as_str());
        let played = t.iter().sum::<u32>().max(1);
        println!(
            "SELFPLAY against {name}: {}-{}-{} (learner wins {:.1}%, score {:.1}%)",
            t[0],
            t[1],
            t[2],
            100.0 * t[0] as f64 / played as f64,
            100.0 * (t[0] as f64 + t[1] as f64 / 2.0) / played as f64
        );
    }
}
