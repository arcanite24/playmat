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
    let result = playmat_ai::mirror::run(&db, stdin.lock(), stdout.lock());
    // this process's CPU, for the server's per-game cost line
    if let Some(ms) = cpu_ms() {
        eprintln!("cpu {ms} ms");
    }
    if let Err(e) = result {
        eprintln!("playmat-ai-bridge: {e}");
        std::process::exit(2);
    }
}

/// User and system CPU time this process has used, in milliseconds
#[cfg(unix)]
fn cpu_ms() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: getrusage fills the struct it's given and reads nothing else
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: getrusage returned 0, so it filled the struct
    let usage = unsafe { usage.assume_init() };
    let ms = |t: libc::timeval| t.tv_sec as u64 * 1000 + t.tv_usec as u64 / 1000;
    Some(ms(usage.ru_utime) + ms(usage.ru_stime))
}

#[cfg(not(unix))]
fn cpu_ms() -> Option<u64> {
    None
}
