//! The prompts and game logs checked in (`playmat-engine/prompts/`, or `$PLAYMAT_PROMPTS`) played through the mirror
//! as Java sends them in a live game (docs/rust/Views.md): seat 1 the AI's, following Java's answers, seat 0's
//! recorded prompts sent at its questions and the log's lines as Java writes them. The copy must count what the
//! engine counts replaying the game on its own: as many prompts compared and differing, as many log lines.

use playmat_ai::mirror::{run_counting, Compared};
use playmat_engine::decide::Query;
use playmat_engine::prompt::prompt_view;
use playmat_engine::replay::{replay_checked, ShadowGame, ShadowPrompt};
use playmat_engine::state::Game;
use playmat_engine::view::compare;
use serde_json::{json, Value};
use std::cell::Cell;
use std::path::PathBuf;

#[test]
fn the_mirror_compares_prompts_and_the_log() {
    let dir = std::env::var("PLAYMAT_PROMPTS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../playmat-engine/prompts"));
    let db = playmat_engine::cards();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|f| f.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut checked = 0;
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let shadow = ShadowGame::parse(&text).unwrap();
        let raw: Value = serde_json::from_str(&text).unwrap();
        if shadow.log.is_empty() || shadow.card_names().iter().any(|n| db.get(n).is_none()) {
            continue;
        }
        // the engine on its own: seat 0's prompts, and those that differ; the log's lines, and those that differ
        let (alone, alone_differ) = (Cell::new(0u64), Cell::new(0u64));
        let on_prompt = |game: &Game, q: &Query, p: &ShadowPrompt| {
            if p.seat == 0 {
                alone.set(alone.get() + 1);
                if prompt_view(game, 0, q).is_none_or(|r| !compare(&p.prompt, &r).is_empty()) {
                    alone_differ.set(alone_differ.get() + 1);
                }
            }
        };
        let (report, rust_log) = replay_checked(&shadow, &db, None, Some(Box::new(on_prompt)));
        if !report.complete() {
            continue;
        }
        // (what Java logged before the first answer replayed happened before the game the copy starts from)
        let first = shadow.start.decision;
        // answer by answer
        let mut by: std::collections::BTreeMap<usize, (Vec<&str>, Vec<&str>)> = Default::default();
        for l in shadow.log.iter().filter(|l| l.decision > first) {
            by.entry(l.decision).or_default().0.push(&l.line);
        }
        for (d, l) in &rust_log {
            by.entry(*d).or_default().1.push(l);
        }
        let (mut java_lines, mut log_differ) = (0u64, 0u64);
        for (_, (java, mut rust)) in by {
            for j in java {
                java_lines += 1;
                match rust.iter().position(|r| *r == j) {
                    Some(i) => {
                        rust.remove(i);
                    }
                    None => log_differ += 1,
                }
            }
        }
        // as Java sends it
        let mut start = json!({"type": "start", "seat": 1, "start": raw["start"], "log": true});
        start["start"] = raw["start"].clone();
        let mut lines = vec![start.to_string()];
        let mut t = 0;
        for k in first..shadow.decisions.len() {
            while t < shadow.trace.len() && shadow.trace[t].decision.is_some_and(|d| d <= k) {
                lines.push(json!({"type": "trace", "fp": shadow.trace[t].fp}).to_string());
                t += 1;
            }
            for l in shadow.log.iter().filter(|l| l.decision == k && k > first) {
                lines.push(json!({"type": "log", "line": l.line}).to_string());
            }
            for p in shadow.prompts.iter().filter(|p| p.decision == k && p.seat == 0) {
                lines.push(json!({"type": "prompt", "prompt": p.prompt}).to_string());
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
        for l in shadow.log.iter().filter(|l| l.decision >= shadow.decisions.len()) {
            lines.push(json!({"type": "log", "line": l.line}).to_string());
        }
        lines.push(json!({"type": "end"}).to_string());
        let mut compared = Compared::default();
        let mut out = Vec::new();
        let result = run_counting(&db, lines.join("\n").as_bytes(), &mut out, &mut compared);
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        assert!(result.is_ok(), "{name}: {result:?}");
        assert!(!String::from_utf8_lossy(&out).contains("\"diverged\""), "{name} left the mirror");
        assert_eq!(
            (compared.prompts.compared, compared.prompts.differing),
            (alone.get(), alone_differ.get()),
            "{name}: the mirror's prompts (compared, differing) aren't the replay's; fields {:?}",
            compared.prompts.fields
        );
        assert_eq!(
            (compared.log.compared, compared.log.differing),
            (java_lines, log_differ),
            "{name}: the mirror's log lines (compared, differing) aren't the replay's; {:?}",
            compared.log.fields
        );
        checked += 1;
    }
    assert!(checked > 0, "no complete games with a log in {}", dir.display());
}
