//! The views checked in (`playmat-engine/views/`, or `$PLAYMAT_VIEWS`) played through the mirror as Java sends them
//! in a live game (docs/rust/Views.md): seat 1 the AI's, following Java's answers, and seat 0's recorded views sent at
//! its questions. The copy must compare each of seat 0's views at the point it was recorded: as many differ as when
//! the engine replays the game on its own.

use playmat_ai::mirror::{run_counting, Compared};
use playmat_engine::replay::{replay_with_views, ShadowGame, ShadowView};
use playmat_engine::state::Game;
use playmat_engine::view::{compare, game_view};
use serde_json::{json, Value};
use std::cell::Cell;
use std::path::PathBuf;

#[test]
fn the_mirror_compares_the_other_seats_views() {
    let dir = std::env::var("PLAYMAT_VIEWS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../playmat-engine/views"));
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
        if shadow.card_names().iter().any(|n| db.get(n).is_none()) {
            continue;
        }
        // the engine on its own: seat 0's views, and those that differ
        let (alone, alone_differ) = (Cell::new(0u64), Cell::new(0u64));
        let on_view = |game: &Game, v: &ShadowView| {
            if v.seat == 0 {
                alone.set(alone.get() + 1);
                if !compare(&v.view, &game_view(game, 0, &["A", "B"])).is_empty() {
                    alone_differ.set(alone_differ.get() + 1);
                }
            }
        };
        let report = replay_with_views(&shadow, &db, Some(Box::new(on_view)));
        if !report.complete() {
            continue;
        }
        // as Java sends it
        let mut lines = vec![json!({"type": "start", "seat": 1, "start": raw["start"]}).to_string()];
        let mut t = 0;
        for k in shadow.start.decision..shadow.decisions.len() {
            while t < shadow.trace.len() && shadow.trace[t].decision.is_some_and(|d| d <= k) {
                lines.push(json!({"type": "trace", "fp": shadow.trace[t].fp}).to_string());
                t += 1;
            }
            for v in shadow.views.iter().filter(|v| v.decision == k && v.seat == 0) {
                lines.push(json!({"type": "view", "view": v.view, "names": ["A", "B"]}).to_string());
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
        let mut compared = Compared::default();
        let mut out = Vec::new();
        let result = run_counting(&db, lines.join("\n").as_bytes(), &mut out, &mut compared);
        let views = compared.views;
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        assert!(result.is_ok(), "{name}: {result:?}");
        assert!(!String::from_utf8_lossy(&out).contains("\"diverged\""), "{name} left the mirror");
        assert_eq!(
            (views.compared, views.differing),
            (alone.get(), alone_differ.get()),
            "{name}: the mirror's views (compared, differing) aren't the replay's; fields {:?}",
            views.fields
        );
        checked += 1;
    }
    assert!(checked > 0, "no games with views in {}", dir.display());
}
