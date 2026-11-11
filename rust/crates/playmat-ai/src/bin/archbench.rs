//! Archbench, the versioned AI benchmark (archbench/README.md), on the Rust engine: a suite's seeded deals played
//! round robin between entrants, each deal twice with the entrants swapped (duplicate pairing), Bradley–Terry
//! ratings anchored at `random` = 1000.
//!
//! cargo run --release -p playmat-ai --bin archbench -- [--suite archbench/constructed-1]
//!     [--entrants rust-playouts@s4,rust-mcts@s4,rust-quick,random] [--deals 84] [--jobs 3] [--record]
//!
//! `--record` appends the run to the suite's results.jsonl; it refuses when the suite's decks changed from its lock.

use playmat_ai::decision::PolicyKind;
use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::eval;
use playmat_ai::search::{Config, Table};
use playmat_engine::card::CardDb;
use playmat_engine::rules;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// An entrant: a name and the seat's config (`None`: the quick answers alone)
#[derive(Clone)]
struct Entrant {
    name: String,
    config: Option<Config>,
}

/// The entrants Archbench knows. Names are versioned with what they play like: a change to an entrant's search
/// that changes its games gets a new name (or a new `@v`), so recorded results keep meaning what they said.
fn entrant(name: &str) -> Result<Entrant, String> {
    let full = Config { threads: 4, policy: PolicyKind::Playouts, ..Config::default() };
    // the Rust host's skill 4 (`playmat-host` `ai_config`): half the budget, 8 candidates
    let s4 = Config { budget: full.budget / 2, width: 8, ..full.clone() };
    let config = match name {
        "rust-playouts@s4" => Some(s4),
        "rust-playouts@full" => Some(full),
        "rust-mcts@s4" => Some(Config { policy: PolicyKind::Mcts, ..s4 }),
        "rust-mcts@full" => Some(Config { policy: PolicyKind::Mcts, ..full }),
        "rust-quick" => None,
        "random" => Some(Config { policy: PolicyKind::Random, ..full }),
        // for tuning, not recording: try:<policy>:<budget>:<width>:<extra turns>:<threads>
        _ if name.starts_with("try:") => {
            let p: Vec<&str> = name.split(':').collect();
            let num = |i: usize, d: u64| p.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
            Some(Config {
                policy: p.get(1).and_then(|v| v.parse().ok()).unwrap_or_default(),
                budget: num(2, s4.budget),
                width: num(3, s4.width as u64) as usize,
                extra_turns: num(4, 0) as u32,
                threads: num(5, 4) as usize,
                ..s4
            })
        }
        _ => return Err(format!("unknown entrant {name}")),
    };
    Ok(Entrant { name: name.to_string(), config })
}

#[derive(Clone, Copy, Default)]
struct Score {
    wins: u32,
    draws: u32,
    losses: u32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let repo = env!("CARGO_MANIFEST_DIR").to_string() + "/../../..";
    let suite_dir = opt("--suite").unwrap_or_else(|| format!("{repo}/archbench/constructed-1"));
    let suite: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{suite_dir}/suite.json")).expect("suite.json in the suite folder"),
    )
    .expect("suite.json");
    let names: Vec<String> = opt("--entrants")
        .unwrap_or_else(|| "rust-playouts@s4,rust-mcts@s4,rust-quick,random".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let entrants: Vec<Entrant> = names.iter().map(|n| entrant(n).unwrap_or_else(|e| panic!("{e}"))).collect();
    let turns = suite["turn_cap"].as_u64().unwrap_or(30) as u32;
    let jobs: usize = opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or(3);

    // the decks, checked against the lock
    let mut decks: Vec<Deck> = Vec::new();
    let mut changed = Vec::new();
    for d in suite["decks"].as_array().expect("decks") {
        let path = d["file"].as_str().unwrap();
        let text = std::fs::read_to_string(format!("{repo}/{path}")).unwrap_or_else(|_| panic!("deck {path}"));
        if format!("{:016x}", fnv(&text)) != d["fnv64"].as_str().unwrap_or("") {
            changed.push(path.to_string());
        }
        let stem = std::path::Path::new(path).file_stem().unwrap().to_str().unwrap();
        decks.push(Deck::parse(stem, &text));
    }
    let n = decks.len();
    let deals: usize =
        opt("--deals").and_then(|v| v.parse().ok()).unwrap_or(suite["deals"].as_u64().unwrap_or(84) as usize);
    let db = playmat_engine::cards();
    let playable: Vec<usize> = (0..deals)
        .filter(|i| {
            let (a, b) = pairing(*i, n);
            decks[a].missing(&db).is_empty() && decks[b].missing(&db).is_empty()
        })
        .collect();
    println!(
        "ARCHBENCH {} ({} decks), {} of {} deals playable, entrants {}{}",
        suite["suite"].as_str().unwrap_or("?"),
        n,
        playable.len(),
        deals,
        names.join(", "),
        if changed.is_empty() { String::new() } else { format!(", DECKS CHANGED FROM THE LOCK: {}", changed.join(", ")) }
    );

    // every game: (deal, entrant on seat 0, entrant on seat 1)
    let mut games = Vec::new();
    for x in 0..entrants.len() {
        for y in x + 1..entrants.len() {
            for &i in &playable {
                games.push((i, x, y));
                games.push((i, y, x));
            }
        }
    }
    let k = entrants.len();
    let scores = Mutex::new(vec![vec![Score::default(); k]; k]);
    let worst = Mutex::new(vec![0u128; k]);
    let decisions = Mutex::new(vec![0u64; k]);
    let failed = AtomicUsize::new(0);
    let next = AtomicUsize::new(0);
    let start = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| loop {
                let g = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(i, p0, p1)) = games.get(g) else { break };
                let (a, b) = pairing(i, n);
                let r = play(&db, [&decks[a], &decks[b]], i, [&entrants[p0], &entrants[p1]], turns);
                let Some((winner, stats)) = r else {
                    failed.fetch_add(1, Ordering::Relaxed);
                    println!("ARCHBENCH deal {i} {} vs {}: failed", entrants[p0].name, entrants[p1].name);
                    continue;
                };
                {
                    let mut sc = scores.lock().unwrap();
                    match winner {
                        Some(0) => {
                            sc[p0][p1].wins += 1;
                            sc[p1][p0].losses += 1;
                        }
                        Some(_) => {
                            sc[p1][p0].wins += 1;
                            sc[p0][p1].losses += 1;
                        }
                        None => {
                            sc[p0][p1].draws += 1;
                            sc[p1][p0].draws += 1;
                        }
                    }
                }
                let mut w = worst.lock().unwrap();
                let mut c = decisions.lock().unwrap();
                for (seat, p) in [p0, p1].into_iter().enumerate() {
                    w[p] = w[p].max(stats[seat].0);
                    c[p] += stats[seat].1;
                }
            });
        }
    });
    let secs = start.elapsed().as_secs_f64();
    let scores = scores.into_inner().unwrap();
    let worst = worst.into_inner().unwrap();
    let decisions = decisions.into_inner().unwrap();
    let ratings = bradley_terry(&scores, names.iter().position(|n| n == "random"));

    println!("ARCHBENCH {} games in {secs:.0} s ({} failed)", games.len(), failed.load(Ordering::Relaxed));
    let mut order: Vec<usize> = (0..k).collect();
    order.sort_by(|a, b| ratings[*b].total_cmp(&ratings[*a]));
    for &x in &order {
        let t = scores[x].iter().fold(Score::default(), |t, s| Score {
            wins: t.wins + s.wins,
            draws: t.draws + s.draws,
            losses: t.losses + s.losses,
        });
        println!(
            "ARCHBENCH {:<20} rating {:>6.0}  won {:>4} drew {:>3} lost {:>4}  worst decision {} ms over {} decisions",
            names[x],
            ratings[x],
            t.wins,
            t.draws,
            t.losses,
            worst[x] / 1_000_000,
            decisions[x]
        );
    }
    for &x in &order {
        for &y in &order {
            if x != y {
                let s = scores[x][y];
                println!("ARCHBENCH   {} vs {}: {}-{}-{}", names[x], names[y], s.wins, s.draws, s.losses);
            }
        }
    }

    if args.iter().any(|a| a == "--record") {
        assert!(changed.is_empty(), "the suite's decks changed from its lock: make a new suite version");
        let commit = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let entry = json!({
            "suite": suite["suite"],
            "date": opt("--date").unwrap_or_default(),
            "commit": commit,
            "harness": "rust-engine",
            "deals": playable.len(),
            "games": games.len(),
            "failed": failed.load(Ordering::Relaxed),
            "entrants": (0..k).map(|x| {
                let t = scores[x].iter().fold(Score::default(), |t, s| Score {
                    wins: t.wins + s.wins, draws: t.draws + s.draws, losses: t.losses + s.losses });
                json!({"name": names[x], "rating": ratings[x].round(), "won": t.wins, "drew": t.draws,
                       "lost": t.losses, "worst_decision_ms": (worst[x] / 1_000_000) as u64})
            }).collect::<Vec<_>>(),
            "pairs": (0..k).flat_map(|x| (x + 1..k).map(move |y| (x, y))).map(|(x, y)| {
                let s = scores[x][y];
                json!({"a": names[x], "b": names[y], "a_won": s.wins, "drew": s.draws, "b_won": s.losses})
            }).collect::<Vec<_>>(),
        });
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{suite_dir}/results.jsonl"))
            .expect("results.jsonl");
        writeln!(f, "{entry}").unwrap();
        println!("ARCHBENCH recorded in {suite_dir}/results.jsonl");
    }
}

/// `AiSpeedBenchTest`'s pairings: deal i is deck i % n against deck (i % n + 1 + i / n % (n - 1)) % n
fn pairing(i: usize, n: usize) -> (usize, usize) {
    let a = i % n;
    (a, (a + 1 + (i / n) % (n - 1)) % n)
}

/// One game: the winning seat (`None` for a draw) and each seat's (worst decision ns, decisions), or `None` when
/// the game failed (an engine panic, or cut off)
fn play(db: &CardDb, decks: [&Deck; 2], deal: usize, who: [&Entrant; 2], turns: u32) -> Option<(Option<usize>, [(u128, u64); 2])> {
    let mut game = new_duel(db, decks, deal as i64, deal % 2).ok()?;
    let mut table = Table::new(vec![who[0].config.clone(), who[1].config.clone()]);
    let run = catch_unwind(AssertUnwindSafe(|| rules::play_from_priority(&mut game, &mut table, turns)));
    if run.is_err() || table.stopped_early() {
        return None;
    }
    let winner = if game.player(0).lost || eval::evaluate(&game, 1) == eval::WIN {
        Some(1)
    } else if game.player(1).lost || eval::evaluate(&game, 0) == eval::WIN {
        Some(0)
    } else {
        None
    };
    let mut stats = [(0u128, 0u64); 2];
    for s in &table.stats {
        let e = &mut stats[s.seat.min(1)];
        e.0 = e.0.max(s.nanos);
        e.1 += 1;
    }
    Some((winner, stats))
}

/// Bradley–Terry strengths by minorization–maximization (a draw is half a win each way), on the Elo scale with the
/// anchor at 1000 (or the mean at 1000 without one)
fn bradley_terry(s: &[Vec<Score>], anchor: Option<usize>) -> Vec<f64> {
    let k = s.len();
    // a half game of prior against each other entrant keeps an unbeaten entrant finite
    let w = |x: usize, y: usize| s[x][y].wins as f64 + s[x][y].draws as f64 / 2.0 + 0.5;
    let mut p = vec![1.0f64; k];
    for _ in 0..2000 {
        let mut q = p.clone();
        for x in 0..k {
            let wins: f64 = (0..k).filter(|y| *y != x).map(|y| w(x, y)).sum();
            let denom: f64 = (0..k).filter(|y| *y != x).map(|y| (w(x, y) + w(y, x)) / (p[x] + p[y])).sum();
            q[x] = wins / denom;
        }
        let g: f64 = q.iter().map(|v| v.ln()).sum::<f64>() / k as f64;
        p = q.iter().map(|v| (v.ln() - g).exp()).collect();
    }
    let elo: Vec<f64> = p.iter().map(|v| 400.0 * v.log10()).collect();
    let zero = anchor.map(|a| elo[a]).unwrap_or(elo.iter().sum::<f64>() / k as f64);
    elo.iter().map(|e| e - zero + 1000.0).collect()
}

/// FNV-1a, 64 bits: the suite lock's fingerprint of a deck file
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}
