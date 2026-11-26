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
//! job goes in `--out` (`selfplay::Shard`), with
//! `wNN.answers.jsonl`: each game's full answer log (`answerlog`), which `playmat-worker verify` replays.
//!
//! Learners: any Archbench entrant (`host@s6` records the search AI, for imitation), or the network:
//! `cfg:policy=net+net=PATH+temp=1000` samples its policy, `cfg:policy=netsearch+hide=1+samples=4+net=PATH` searches.

use playmat_ai::entrants::{entrant, Entrant};
use playmat_ai::play;
use playmat_ai::selfplay::Shard;
use serde_json::json;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    // deck files are named from the repository's root; `--repo` moves it (the image has only the suite's decks)
    let repo = opt("--repo").unwrap_or_else(|| env!("CARGO_MANIFEST_DIR").to_string() + "/../../..");
    let suite_dir = opt("--suite").unwrap_or_else(|| format!("{repo}/archbench/constructed-2"));
    let (suite, decks) = play::load_suite(&repo, &suite_dir).unwrap_or_else(|e| panic!("{e}"));
    let learner = entrant(&opt("--learner").unwrap_or_else(|| "host@s6".into())).unwrap_or_else(|e| panic!("{e}"));
    let mut opponents: Vec<(Option<Entrant>, u64)> = Vec::new();
    for part in opt("--opponents").unwrap_or_else(|| "self".into()).split(';').filter(|s| !s.is_empty()) {
        let (name, w) = part.rsplit_once('*').map_or((part, 1), |(a, b)| (a, b.parse().unwrap_or(1)));
        let e = (name != "self").then(|| entrant(name).unwrap_or_else(|e| panic!("{e}")));
        opponents.push((e, w));
    }
    let weights: Vec<u64> = opponents.iter().map(|o| o.1).collect();
    let games: usize = opt("--games").and_then(|v| v.parse().ok()).unwrap_or(2000);
    let jobs: usize = opt("--jobs").and_then(|v| v.parse().ok()).unwrap_or(10);
    let keep: u32 = opt("--keep").and_then(|v| v.parse().ok()).unwrap_or(1000);
    let (fast, fast_iters) =
        opt("--fast").map_or((None, None), |v| play::parse_fast(&v).unwrap_or_else(|e| panic!("{e}")));
    let rec = play::Recorder { keep, fast, fast_iters };
    let seed: usize = opt("--seed").and_then(|v| v.parse().ok()).unwrap_or(0);
    let tag: i32 = opt("--tag").and_then(|v| v.parse().ok()).unwrap_or(0);
    let out = opt("--out").expect("--out DIR");
    let db = playmat_engine::cards();
    let setup = play::Setup {
        db: &db,
        decks: &decks,
        turns: suite["turn_cap"].as_u64().unwrap_or(30) as u32,
        mulligans: suite["mulligans"].as_bool().unwrap_or(false),
    };

    let started = std::time::Instant::now();
    let next = AtomicUsize::new(0);
    // per opponent: learner wins, draws, losses
    let tally: Mutex<Vec<[u32; 3]>> = Mutex::new(vec![[0; 3]; opponents.len()]);
    let steps = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for job in 0..jobs {
            let (setup, learner, opponents, weights, tally, steps, failed, next, out) =
                (&setup, &learner, &opponents, &weights, &tally, &steps, &failed, &next, &out);
            s.spawn(move || {
                let name = format!("w{job:02}");
                let mut shard = Shard::create(out, &name).expect("shard");
                // every game's answers, so it can be replayed without its searches (playmat-worker verify)
                let mut answers = std::io::BufWriter::new(
                    std::fs::File::create(format!("{out}/{name}.answers.jsonl")).expect("answers"),
                );
                loop {
                    let g = next.fetch_add(1, Ordering::Relaxed);
                    if g >= games {
                        break;
                    }
                    let deal = seed + g;
                    let p = play::pairing(deal, setup.decks.len(), weights);
                    let oi = p.opponent;
                    let opp = opponents[oi].0.as_ref().unwrap_or(learner);
                    let me = p.me;
                    let mut configs = [opp.config.clone(), opp.config.clone()];
                    configs[me] = learner.config.clone();
                    let record = [opponents[oi].0.is_none() || me == 0, opponents[oi].0.is_none() || me == 1];
                    let run = catch_unwind(AssertUnwindSafe(|| play::play_game(setup, deal, p, configs, record, rec)));
                    let Ok(Ok(played)) = run else {
                        failed.fetch_add(1, Ordering::Relaxed);
                        continue;
                    };
                    let result = match played.winner {
                        Some(w) => [(w == 0) as u8 as f32, (w == 1) as u8 as f32],
                        None => [0.5, 0.5],
                    };
                    {
                        let mut t = tally.lock().unwrap();
                        let k = match played.winner {
                            Some(w) if w == me => 0,
                            None => 1,
                            _ => 2,
                        };
                        t[oi][k] += 1;
                    }
                    steps.fetch_add(played.steps.len(), Ordering::Relaxed);
                    let [a, b] = p.decks;
                    let info = json!({
                        "deal": deal, "decks": [setup.decks[a].name, setup.decks[b].name], "learner": me,
                        "opponent": opp.name, "turns": played.turns, "first": p.first,
                    });
                    shard.game(deal, tag, &played.steps, result, info).expect("write");
                    let line = json!({
                        "deal": deal, "winner": played.winner, "turns": played.turns,
                        "answers": played.log.entries.len(), "recorded": played.steps.len(),
                        "log_sha256": played.log.hash(), "steps_sha256": play::steps_hash(&played.steps),
                        "log": played.log.encode(),
                    });
                    writeln!(answers, "{line}").expect("answers");
                    if g % 500 == 0 {
                        let _ = writeln!(
                            std::io::stderr(),
                            "SELFPLAY {g}/{games} games, {} steps, {:.0} s",
                            steps.load(Ordering::Relaxed),
                            started.elapsed().as_secs_f64()
                        );
                    }
                }
                shard.flush().expect("flush");
                answers.flush().expect("answers");
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
