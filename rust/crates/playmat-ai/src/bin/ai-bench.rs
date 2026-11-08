//! AI speed bench for the Rust AI: the same seeded duels as Java's `AiSpeedBenchTest` (the Pioneer and Standard
//! school decks, game i is deck i % n against deck (i % n + 1 + i / n % (n - 1)) % n, seat A starts on even games),
//! both seats played by the Rust AI. Prints the same BENCH lines: time per game, the decision percentiles and the
//! slowest decisions.
//!
//! cargo run --release -p playmat-ai --bin ai-bench -- [--games 24] [--first 0] [--only 1,5] [--turns 30]
//!     [--budget 6000] [--width 12] [--extra-turns 0] [--schools DIR,DIR]

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::eval;
use playmat_ai::search::{Config, DecisionStat, Table};
use playmat_engine::rules;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let num = |name: &str, d: u64| opt(name).and_then(|v| v.parse().ok()).unwrap_or(d);
    let games = num("--games", 24) as usize;
    let first = num("--first", 0) as usize;
    let turns = num("--turns", 30) as u32;
    let cfg = Config {
        budget: num("--budget", Config::default().budget),
        width: num("--width", Config::default().width as u64) as usize,
        extra_turns: num("--extra-turns", 0) as u32,
        playout_cap: num("--playout-cap", Config::default().playout_cap),
    };
    // `--b-…`: seat B plays with these instead (to compare two configs; run again with `--swap` for the other seats)
    let cfg_b = Config {
        budget: num("--b-budget", cfg.budget),
        width: num("--b-width", cfg.width as u64) as usize,
        extra_turns: num("--b-extra-turns", cfg.extra_turns as u64) as u32,
        playout_cap: num("--b-playout-cap", cfg.playout_cap),
    };
    let (cfg, cfg_b) = if args.iter().any(|a| a == "--swap") { (cfg_b, cfg) } else { (cfg, cfg_b) };
    let root = env!("CARGO_MANIFEST_DIR").to_string() + "/../../../Mage.Server/career-src";
    let schools = opt("--schools").unwrap_or_else(|| format!("{root}/pioneer,{root}/standard"));
    let mut decks: Vec<Deck> = Vec::new();
    for dir in schools.split(',') {
        let mut files: Vec<_> = std::fs::read_dir(dir).expect("deck folder").flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files.into_iter().filter(|f| f.extension().is_some_and(|e| e == "txt")) {
            let text = std::fs::read_to_string(&f).unwrap();
            decks.push(Deck::parse(f.file_stem().unwrap().to_str().unwrap(), &text));
        }
    }
    let db = playmat_engine::cards();
    let indexes: Vec<usize> = match opt("--only") {
        Some(list) => list.split(',').filter_map(|i| i.trim().parse().ok()).collect(),
        None => (first..first + games).collect(),
    };
    let n = decks.len();
    println!("BENCH {} decks, Rust AI {:?}, seat B {:?}, draw at turn {}", n, cfg, cfg_b, turns);
    let mut all: Vec<DecisionStat> = Vec::new();
    let (mut total, mut played, mut failed) = (0.0f64, 0usize, 0usize);
    let mut wins = [0usize; 3];
    for i in indexes {
        let a = i % n;
        let b = (a + 1 + (i / n) % (n - 1)) % n;
        let missing: Vec<String> = decks[a].missing(&db).into_iter().chain(decks[b].missing(&db)).collect();
        if !missing.is_empty() {
            println!(
                "BENCH game {i} {} vs {}: skipped, not in the engine: {}",
                decks[a].name,
                decks[b].name,
                missing.join(", ")
            );
            continue;
        }
        let start = Instant::now();
        let mut game = new_duel(&db, [&decks[a], &decks[b]], i as i64, i % 2).unwrap();
        let mut table = Table::new(vec![Some(cfg.clone()), Some(cfg_b.clone())]);
        let run = catch_unwind(AssertUnwindSafe(|| rules::play_from_priority(&mut game, &mut table, turns)));
        let secs = start.elapsed().as_secs_f64();
        let mut ms: Vec<u128> = table.stats.iter().map(|s| s.nanos / 1_000_000).collect();
        ms.sort();
        let outcome = match run {
            Err(e) => {
                failed += 1;
                let why = e.downcast_ref::<String>().cloned().or(e.downcast_ref::<&str>().map(|s| s.to_string()));
                format!("engine panic at turn {}: {}", game.turn, why.unwrap_or_default())
            }
            Ok(()) if table.stopped_early() => {
                failed += 1;
                format!("stopped after {} answers at turn {}", table.answers, game.turn)
            }
            Ok(()) => {
                played += 1;
                let w = if game.player(0).lost || eval::evaluate(&game, 1) == eval::WIN {
                    1
                } else if game.player(1).lost || eval::evaluate(&game, 0) == eval::WIN {
                    0
                } else {
                    2
                };
                wins[w] += 1;
                match w {
                    0 => format!("{} won on turn {}", decks[a].name, game.turn),
                    1 => format!("{} won on turn {}", decks[b].name, game.turn),
                    _ => format!("draw at turn {}", game.turn),
                }
            }
        };
        total += secs;
        println!(
            "BENCH game {i} {} vs {}: {outcome}, {secs:.2} s, {} decisions, p95 {} ms, max {} ms",
            decks[a].name,
            decks[b].name,
            ms.len(),
            pct(&ms, 0.95),
            ms.last().copied().unwrap_or(0)
        );
        for s in table.stats.iter_mut() {
            s.game = i;
        }
        all.extend(table.stats);
    }
    let mut ms: Vec<u128> = all.iter().map(|s| s.nanos / 1_000_000).collect();
    ms.sort();
    let nodes: u64 = all.iter().map(|s| s.nodes).sum();
    let search: f64 = all.iter().map(|s| s.nanos as f64 / 1e9).sum();
    let games_run = (played + failed).max(1);
    println!(
        "BENCH total: {} games ({} finished, {} failed), {:.2} s per game (one thread), searches {:.1} s, {} nodes, {:.1} us per node; seat A won {}, seat B won {}, draws {}",
        played + failed,
        played,
        failed,
        total / games_run as f64,
        search,
        nodes,
        if nodes == 0 { 0.0 } else { search * 1e6 / nodes as f64 },
        wins[0],
        wins[1],
        wins[2]
    );
    println!(
        "BENCH decisions: {}, p50 {} ms, p90 {} ms, p95 {} ms, p99 {} ms, max {} ms, over 1 s {}, over 3 s {}",
        ms.len(),
        pct(&ms, 0.5),
        pct(&ms, 0.9),
        pct(&ms, 0.95),
        pct(&ms, 0.99),
        ms.last().copied().unwrap_or(0),
        ms.iter().filter(|t| **t > 1000).count(),
        ms.iter().filter(|t| **t > 3000).count()
    );
    all.sort_by_key(|s| std::cmp::Reverse(s.nanos));
    for s in all.iter().take(10) {
        println!(
            "BENCH slow: game {} seat {} turn {} {}, {} ms, {} nodes, {} options, {} permanents",
            s.game,
            s.seat,
            s.turn,
            s.kind,
            s.nanos / 1_000_000,
            s.nodes,
            s.options,
            s.permanents
        );
    }
}

fn pct(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1]
}
