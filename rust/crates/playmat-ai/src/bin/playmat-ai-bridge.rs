//! The Rust AI for a seat of a game Java hosts (`playmat_ai::mirror`): one game per process, JSON lines on stdin and
//! stdout. `--check` instead reads card names, one per line, and prints those the engine doesn't have.

use std::io::{BufRead, Write};

fn main() {
    let db = playmat_engine::cards();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    if std::env::args().any(|a| a == "--check") {
        let mut out = stdout.lock();
        for name in stdin.lock().lines().map_while(Result::ok) {
            let name = name.trim();
            if !name.is_empty() && db.get(name).is_none() {
                let _ = writeln!(out, "{name}");
            }
        }
        return;
    }
    if let Err(e) = playmat_ai::mirror::run(&db, stdin.lock(), stdout.lock()) {
        eprintln!("playmat-ai-bridge: {e}");
        std::process::exit(2);
    }
}
