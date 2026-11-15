//! Archbench, the versioned AI benchmark (archbench/README.md), on the Rust engine: a suite's seeded deals played
//! round robin between entrants, each deal twice with the entrants swapped (duplicate pairing), Bradley–Terry
//! ratings anchored at `random` = 1000.
//!
//! cargo run --release -p playmat-ai --bin archbench -- [--suite archbench/constructed-2]
//!     [--entrants rust-playouts@s4,rust-mcts@s4,rust-quick,random] [--deals N] [--jobs 3] [--record] [--by-deck]
//!     [--sprt elo0,elo1 [--max-deals 20000]]
//!
//! `--record` appends the run to the suite's results.jsonl; it refuses when the suite's decks changed from its lock.
//! `--sprt` plays two entrants against each other until a sequential test says whether the first is `elo1` Elo
//! better or not `elo0` better (archbench/README.md).

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::entrants::{entrant, Entrant};
use playmat_ai::eval;
use playmat_ai::mulligan;
use playmat_ai::search::Table;
use playmat_engine::card::CardDb;
use playmat_engine::rules;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

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
    let suite_dir = opt("--suite").unwrap_or_else(|| format!("{repo}/archbench/constructed-2"));
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
    let jobs: usize = opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or(3);
    let suite_rules = SuiteRules {
        turns: suite["turn_cap"].as_u64().unwrap_or(30) as u32,
        mulligans: suite["mulligans"].as_bool().unwrap_or(false),
    };

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
    let db = playmat_engine::cards();
    let usable = |i: &usize| {
        let (a, b) = pairing(*i, n);
        decks[a].missing(&db).is_empty() && decks[b].missing(&db).is_empty()
    };

    if let Some(bounds) = opt("--sprt") {
        assert_eq!(entrants.len(), 2, "--sprt compares two entrants");
        let b: Vec<f64> = bounds.split(',').filter_map(|v| v.trim().parse().ok()).collect();
        let (elo0, elo1) = (b.first().copied().unwrap_or(0.0), b.get(1).copied().unwrap_or(10.0));
        let max: usize = opt("--max-deals").and_then(|v| v.parse().ok()).unwrap_or(20_000);
        // in a fixed shuffled order, so a test that stops early has still seen every kind of matchup
        let mut deals: Vec<usize> = (0..max).filter(usable).collect();
        deals.sort_by_key(|i| (*i as u64 ^ 0x9e37_79b9_7f4a_7c15).wrapping_mul(0xbf58_476d_1ce4_e5b9).rotate_left(31));
        sprt(&db, &decks, &deals, &entrants, suite_rules, jobs, elo0, elo1);
        return;
    }

    let deals: usize =
        opt("--deals").and_then(|v| v.parse().ok()).unwrap_or(suite["deals"].as_u64().unwrap_or(84) as usize);
    let playable: Vec<usize> = (0..deals).filter(usable).collect();
    println!(
        "ARCHBENCH {} ({} decks), {} of {} deals playable, entrants {}{}",
        suite["suite"].as_str().unwrap_or("?"),
        n,
        playable.len(),
        deals,
        names.join(", "),
        if changed.is_empty() {
            String::new()
        } else {
            format!(", DECKS CHANGED FROM THE LOCK: {}", changed.join(", "))
        }
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
    // each entrant's results by the deck it played
    let by_deck = Mutex::new(vec![vec![Score::default(); n]; k]);
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
                let r = play(&db, [&decks[a], &decks[b]], i, [&entrants[p0], &entrants[p1]], suite_rules);
                let Some((winner, stats)) = r else {
                    failed.fetch_add(1, Ordering::Relaxed);
                    println!("ARCHBENCH deal {i} {} vs {}: failed", entrants[p0].name, entrants[p1].name);
                    continue;
                };
                {
                    let mut sc = scores.lock().unwrap();
                    let mut bd = by_deck.lock().unwrap();
                    match winner {
                        Some(0) => {
                            sc[p0][p1].wins += 1;
                            sc[p1][p0].losses += 1;
                            bd[p0][a].wins += 1;
                            bd[p1][b].losses += 1;
                        }
                        Some(_) => {
                            sc[p1][p0].wins += 1;
                            sc[p0][p1].losses += 1;
                            bd[p1][b].wins += 1;
                            bd[p0][a].losses += 1;
                        }
                        None => {
                            sc[p0][p1].draws += 1;
                            sc[p1][p0].draws += 1;
                            bd[p0][a].draws += 1;
                            bd[p1][b].draws += 1;
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
    let by_deck = by_deck.into_inner().unwrap();
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
    if args.iter().any(|a| a == "--by-deck") {
        for &x in &order {
            for (d, deck) in decks.iter().enumerate() {
                let s = by_deck[x][d];
                let played = (s.wins + s.draws + s.losses).max(1) as f64;
                println!(
                    "ARCHBENCH deck {:<20} {:<36} {}-{}-{} ({:.0}%)",
                    names[x],
                    deck.name,
                    s.wins,
                    s.draws,
                    s.losses,
                    100.0 * (s.wins as f64 + s.draws as f64 / 2.0) / played
                );
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

/// How a suite's games are played
#[derive(Clone, Copy)]
struct SuiteRules {
    /// the game is a draw when this turn would begin
    turns: u32,
    /// the London mulligan before the first turn (each seat's AI decides)
    mulligans: bool,
}

/// A sequential probability ratio test of entrant 0 against entrant 1: deals in order, each played twice with the
/// seats swapped, until the log-likelihood ratio of "entrant 0 is `elo1` better" against "`elo0` better" crosses a
/// bound (α = β = 0.05) or the deals run out. Each deal's pair of games is one sample (its mean score), so the luck
/// of the deal cancels out; the ratio is the normal approximation fishtest uses (GSPRT on the pair scores).
#[allow(clippy::too_many_arguments)]
fn sprt(
    db: &CardDb,
    decks: &[Deck],
    deals: &[usize],
    entrants: &[Entrant],
    suite_rules: SuiteRules,
    jobs: usize,
    elo0: f64,
    elo1: f64,
) {
    let (alpha, beta) = (0.05f64, 0.05f64);
    let (lower, upper) = ((beta / (1.0 - alpha)).ln(), ((1.0 - beta) / alpha).ln());
    println!(
        "SPRT {} against {}, elo0 {elo0} elo1 {elo1}, bounds {lower:.2} {upper:.2}, up to {} deals",
        entrants[0].name,
        entrants[1].name,
        deals.len()
    );
    let n = decks.len();
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    // the pair scores (0, 0.25, 0.5, 0.75, 1) in the order they finish
    let samples = Mutex::new(Vec::<f64>::new());
    let wdl = Mutex::new([0u32; 3]);
    let start = Instant::now();
    let verdict = Mutex::new(None::<bool>);
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1) {
            s.spawn(|| loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let g = next.fetch_add(1, Ordering::Relaxed);
                let Some(&i) = deals.get(g) else { break };
                let (a, b) = pairing(i, n);
                let mut pair = 0.0;
                let mut ok = true;
                for (p0, p1) in [(0usize, 1usize), (1, 0)] {
                    match play(db, [&decks[a], &decks[b]], i, [&entrants[p0], &entrants[p1]], suite_rules) {
                        Some((winner, _)) => {
                            // entrant 0's score in this game
                            let score = match winner {
                                Some(w) if (w == 0) == (p0 == 0) => 1.0,
                                Some(_) => 0.0,
                                None => 0.5,
                            };
                            let mut t = wdl.lock().unwrap();
                            t[if score == 1.0 {
                                0
                            } else if score == 0.5 {
                                1
                            } else {
                                2
                            }] += 1;
                            pair += score / 2.0;
                        }
                        None => ok = false,
                    }
                }
                if !ok {
                    continue;
                }
                let mut v = samples.lock().unwrap();
                v.push(pair);
                let ratio = llr(&v, elo0, elo1);
                if v.len() % 250 == 0 {
                    let t = wdl.lock().unwrap();
                    println!(
                        "SPRT {} pairs, {}-{}-{}, elo {:+.1}, LLR {ratio:.2}",
                        v.len(),
                        t[0],
                        t[1],
                        t[2],
                        elo_of(&v)
                    );
                }
                if v.len() >= 100 && (ratio <= lower || ratio >= upper) && !stop.swap(true, Ordering::Relaxed) {
                    *verdict.lock().unwrap() = Some(ratio >= upper);
                }
            });
        }
    });
    let v = samples.into_inner().unwrap();
    let t = wdl.into_inner().unwrap();
    let ratio = llr(&v, elo0, elo1);
    let verdict = match verdict.into_inner().unwrap() {
        Some(true) => format!("H1, {} is better (by {elo1} Elo or more)", entrants[0].name),
        Some(false) => format!("H0, {} is not {elo0} Elo better", entrants[0].name),
        None => "no verdict (the deals ran out)".into(),
    };
    let (lo, hi) = elo_interval(&v);
    println!(
        "SPRT result: {verdict}; {} pairs, {}-{}-{}, elo {:+.1} [{lo:+.1}, {hi:+.1}], LLR {ratio:.2}, {:.0} s",
        v.len(),
        t[0],
        t[1],
        t[2],
        elo_of(&v),
        start.elapsed().as_secs_f64()
    );
}

/// The expected score of a player `elo` points stronger
fn score_of(elo: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-elo / 400.0))
}

fn elo_from_score(s: f64) -> f64 {
    let s = s.clamp(1e-6, 1.0 - 1e-6);
    -400.0 * (1.0 / s - 1.0).log10()
}

fn mean_var(v: &[f64]) -> (f64, f64) {
    let n = v.len().max(1) as f64;
    let m = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n;
    (m, var.max(1e-9))
}

/// GSPRT's log-likelihood ratio from the samples' mean and variance
fn llr(v: &[f64], elo0: f64, elo1: f64) -> f64 {
    let (m, var) = mean_var(v);
    let (s0, s1) = (score_of(elo0), score_of(elo1));
    v.len() as f64 * (s1 - s0) * (2.0 * m - s0 - s1) / (2.0 * var)
}

fn elo_of(v: &[f64]) -> f64 {
    elo_from_score(mean_var(v).0)
}

/// A 95% interval on the Elo difference
fn elo_interval(v: &[f64]) -> (f64, f64) {
    let (m, var) = mean_var(v);
    let se = (var / v.len().max(1) as f64).sqrt();
    (elo_from_score(m - 1.96 * se), elo_from_score(m + 1.96 * se))
}

/// `AiSpeedBenchTest`'s pairings: deal i is deck i % n against deck (i % n + 1 + i / n % (n - 1)) % n. The first
/// n·(n-1) deals are every ordered pair of decks once.
fn pairing(i: usize, n: usize) -> (usize, usize) {
    let a = i % n;
    (a, (a + 1 + (i / n) % (n - 1)) % n)
}

/// A seat's worst decision (ns) and its number of decisions
type SeatTiming = (u128, u64);

/// One game: the winning seat (`None` for a draw) and each seat's (worst decision ns, decisions), or `None` when
/// the game failed (an engine panic, or cut off)
fn play(
    db: &CardDb,
    decks: [&Deck; 2],
    deal: usize,
    who: [&Entrant; 2],
    suite_rules: SuiteRules,
) -> Option<(Option<usize>, [SeatTiming; 2])> {
    let mut game = new_duel(db, decks, deal as i64, deal % 2).ok()?;
    let configs = [who[0].config.clone(), who[1].config.clone()];
    let run = catch_unwind(AssertUnwindSafe(|| {
        if suite_rules.mulligans {
            mulligan::london(
                &mut game,
                deal % 2,
                0,
                &mut |g, seat, taken| mulligan::wants(g, seat, taken, configs[seat].as_ref()),
                &mut |g, seat| mulligan::bottom(g, seat, configs[seat].as_ref()),
            );
        }
        let mut table = Table::new(configs.to_vec());
        rules::play_from_priority(&mut game, &mut table, suite_rules.turns);
        table
    }));
    let table = run.ok()?;
    if table.stopped_early() {
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
