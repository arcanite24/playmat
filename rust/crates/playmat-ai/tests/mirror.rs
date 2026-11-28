//! The shadow games checked in, played through the mirror as Java would send them, with seat 1 as the AI's seat
//! following Java's recorded answers: every answer must go back to Java naming what Java chose, and the copy must
//! stay in step with Java's trace to the end.

use playmat_ai::mirror;
use playmat_engine::replay::{resolve, ShadowGame};
use serde_json::{json, Value};
use std::path::PathBuf;

#[test]
fn shadow_games_play_through_the_mirror() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../playmat-engine/shadow");
    let db = playmat_engine::cards();
    let mut files = Vec::new();
    for tier in std::fs::read_dir(&dir).unwrap().flatten() {
        for f in std::fs::read_dir(tier.path()).into_iter().flatten().flatten() {
            if f.path().extension().is_some_and(|e| e == "json") {
                files.push(f.path());
            }
        }
    }
    files.sort();
    let mut failed = Vec::new();
    let mut checked = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let shadow = ShadowGame::parse(&text).unwrap();
        let raw: Value = serde_json::from_str(&text).unwrap();
        if shadow.card_names().iter().any(|n| db.get(n).is_none()) || shadow.result == "timeout" {
            continue;
        }
        let mut lines = vec![json!({"type": "start", "seat": 1, "start": raw["start"]}).to_string()];
        let mut t = 0;
        let first = shadow.start.decision;
        for k in first..shadow.decisions.len() {
            while t < shadow.trace.len() && shadow.trace[t].decision.is_some_and(|d| d <= k) {
                lines.push(json!({"type": "trace", "fp": shadow.trace[t].fp}).to_string());
                t += 1;
            }
            let d = &raw["decisions"][k];
            if shadow.decisions[k].seat == 1 {
                lines.push(json!({"type": "ask", "follow": d}).to_string());
            } else {
                let mut m = d.clone();
                m["type"] = json!("decision");
                lines.push(m.to_string());
            }
        }
        for e in &shadow.trace[t..] {
            lines.push(json!({"type": "trace", "fp": e.fp}).to_string());
        }
        lines.push(json!({"type": "end"}).to_string());
        let input = lines.join("\n");
        let mut out = Vec::new();
        let result = mirror::run(&db, input.as_bytes(), &mut out);
        checked += 1;
        let out = String::from_utf8(out).unwrap();
        let name = f.strip_prefix(&dir).unwrap().display().to_string();
        if let Err(e) = result {
            failed.push(format!("{name}: {e}"));
            continue;
        }
        if out.contains("\"diverged\"") {
            failed.push(format!("{name}: {}", out.lines().last().unwrap_or("")));
        }
        let _ = resolve;
    }
    assert!(checked > 100, "only {checked} games checked");
    assert!(failed.is_empty(), "{} of {checked} games left the mirror:\n{}", failed.len(), failed.join("\n"));
}
