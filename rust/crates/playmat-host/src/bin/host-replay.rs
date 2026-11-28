//! Plays a hosted game's record again (docs/rust/GameHost.md, "Game records") and prints its log:
//!
//!   cargo run --release -p playmat-host --bin host-replay -- <game>.jsonl [--turn N] [--quiet]
//!
//! `--turn N` prints only turn N's lines; `--quiet` prints only the summary. It says whether the replay wrote the
//! record's log and ended the same way; set PLAYMAT_HOST_DEBUG=1 to also see every question and answer.

use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let mut path = None;
    let mut turn = None;
    let mut quiet = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--turn" => turn = args.next().and_then(|t| t.parse::<u32>().ok()),
            "--quiet" => quiet = true,
            _ => path = Some(PathBuf::from(a)),
        }
    }
    let Some(path) = path else {
        eprintln!("usage: host-replay <game>.jsonl [--turn N] [--quiet]");
        std::process::exit(2);
    };
    let rec = match playmat_host::record::read(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let decks: Vec<String> = rec.start.seats.iter().map(|s| format!("{} ({} cards)", s.name, s.deck.len())).collect();
    println!("game {} seed {}: {}; {} calls", rec.start.game_id, rec.start.seed, decks.join(" vs "), rec.calls.len());
    let replayed = match playmat_host::record::replay(&rec, Arc::new(playmat_engine::cards())) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("the replay didn't run: {e}");
            std::process::exit(1);
        }
    };
    if !quiet {
        for (t, line) in &replayed.log {
            if turn.is_none_or(|n| n == *t) {
                println!("T{t}  {}", plain(line));
            }
        }
    }
    let e = &replayed.ended;
    println!("replay: {} after {} turns, winner {:?}", e.reason, e.turns, e.winner);
    if let Some(end) = &rec.end {
        println!("record: {} after {} turns, winner {}", end["reason"], end["turns"], end["winner"]);
    } else {
        println!("record: no end (the game was still on, or the gateway stopped)");
    }
    match &replayed.first_difference {
        None => println!("the replay wrote the record's log ({} lines)", replayed.log.len()),
        Some((i, recorded, replay)) => {
            println!("the logs differ at line {i}:\n  record: {}\n  replay: {}", plain(recorded), plain(replay));
            std::process::exit(1);
        }
    }
}

/// A log line without its HTML
fn plain(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in s.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            c if !tag => out.push(c),
            _ => {}
        }
    }
    out
}
