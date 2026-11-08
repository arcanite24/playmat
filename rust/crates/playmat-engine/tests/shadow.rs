//! Replays the shadow games in `$PLAYMAT_SHADOW` (default `shadow/` in this crate) and checks each matches Java's
//! trace. Games with cards the engine doesn't have, or that Java didn't finish (timeouts, errors), are skipped.
//! The games checked in here must all match; for a folder named by `$PLAYMAT_SHADOW` the test reports and passes,
//! unless `PLAYMAT_SHADOW_STRICT=1`.

use playmat_engine::replay::{replay, ShadowGame};
use std::path::PathBuf;

#[test]
fn shadow_games_match_java() {
    let named = std::env::var("PLAYMAT_SHADOW").ok();
    let strict = named.is_none() || std::env::var("PLAYMAT_SHADOW_STRICT").map(|v| v == "1").unwrap_or(false);
    let dir = named.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("shadow"));
    let db = playmat_engine::cards();
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.sort();
    let (mut matched, mut skipped, mut failed) = (0, 0, Vec::new());
    for f in &files {
        let shadow = ShadowGame::parse(&std::fs::read_to_string(f).unwrap()).unwrap();
        let name = f.strip_prefix(&dir).unwrap_or(f).display().to_string();
        let missing: Vec<String> = shadow.card_names().into_iter().filter(|n| db.get(n).is_none()).collect();
        if !missing.is_empty() || shadow.result == "timeout" || shadow.result.starts_with("error") {
            skipped += 1;
            continue;
        }
        let report = replay(&shadow, &db);
        if report.complete() {
            matched += 1;
            eprintln!("{}: matches ({} entries, {} answers)", name, report.entries, report.answers);
        } else {
            eprintln!(
                "{}: {} of {} entries match, {} answers used\n{}\n",
                name,
                report.matched,
                report.entries,
                report.answers,
                report.failure.clone().unwrap_or_default()
            );
            failed.push(name);
        }
    }
    eprintln!("{} match, {} differ, {} skipped", matched, failed.len(), skipped);
    if strict {
        assert!(failed.is_empty(), "games that differ from Java: {:?}", failed);
    }
}

/// The `.json` files in a folder and its subfolders.
fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().map(|x| x == "json").unwrap_or(false) {
            out.push(p);
        }
    }
}
