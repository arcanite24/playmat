//! Views (docs/rust/Views.md): replays the shadow games with recorded views in `$PLAYMAT_VIEWS` (default `views/` in
//! this crate), renders the engine's view at each and compares it field by field with Java's. The games checked in
//! must match where their state does; for a folder named by `$PLAYMAT_VIEWS` the test reports the differing fields,
//! most common first, and passes unless `PLAYMAT_VIEWS_STRICT=1`. `PLAYMAT_VIEWS_SHOW=<n>` prints n examples a field;
//! `PLAYMAT_VIEWS_DUMP=<file>:<answer>` prints Rust's whole view at one answer.

use playmat_engine::replay::{replay_with_views, ShadowGame, ShadowView};
use playmat_engine::state::Game;
use playmat_engine::view::{compare, field_of, game_view, Difference};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A field's count of differing views, with a few examples (where, and the difference)
type Tally = (usize, Vec<(String, Difference)>);

#[test]
fn views_match_java() {
    let named = std::env::var("PLAYMAT_VIEWS").ok();
    let strict = named.is_none() || std::env::var("PLAYMAT_VIEWS_STRICT").map(|v| v == "1").unwrap_or(false);
    let show: usize = std::env::var("PLAYMAT_VIEWS_SHOW").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let dir = named.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("views"));
    let db = playmat_engine::cards();
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.sort();
    // by field: how many views differ in it, and a few examples
    let fields: RefCell<BTreeMap<String, Tally>> = RefCell::new(BTreeMap::new());
    let (mut views, mut same) = (0usize, 0usize);
    for f in &files {
        let shadow = ShadowGame::parse(&std::fs::read_to_string(f).unwrap()).unwrap();
        if shadow.views.is_empty() || shadow.card_names().iter().any(|n| db.get(n).is_none()) {
            continue;
        }
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let results: RefCell<Vec<Vec<Difference>>> = RefCell::new(Vec::new());
        let dump = std::env::var("PLAYMAT_VIEWS_DUMP").ok();
        let on_view = |game: &Game, v: &ShadowView| {
            let rust = game_view(game, v.seat, &["A", "B"]);
            // `PLAYMAT_VIEWS_DUMP=game-00003.json:75` prints Rust's view at that answer
            if dump.as_deref() == Some(&format!("{}:{}", name, v.decision)) {
                eprintln!("RUST VIEW {}", rust);
            }
            let d = compare(&v.view, &rust);
            let mut seen = Vec::new();
            for x in &d {
                let field = field_of(&x.path);
                if seen.contains(&field) {
                    continue;
                }
                seen.push(field.clone());
                let mut m = fields.borrow_mut();
                let e = m.entry(field).or_default();
                e.0 += 1;
                if e.1.len() < show {
                    e.1.push((format!("{} answer {}", name, v.decision), x.clone()));
                }
            }
            results.borrow_mut().push(d);
        };
        let report = replay_with_views(&shadow, &db, Some(Box::new(on_view)));
        let r = results.into_inner();
        views += r.len();
        same += r.iter().filter(|d| d.is_empty()).count();
        if !report.complete() {
            eprintln!(
                "{}: the state differs from Java's after {} answers; views before it compared",
                name, report.answers
            );
        }
    }
    let mut rows: Vec<(String, Tally)> = fields.into_inner().into_iter().collect();
    rows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(&b.0)));
    for (field, (n, examples)) in &rows {
        eprintln!("{:6}  {}", n, field);
        for (at, d) in examples {
            eprintln!("          {} {}: Java {} | Rust {}", at, d.path, d.java, d.rust);
        }
    }
    eprintln!("{} of {} views match; {} fields differ", same, views, rows.len());
    if strict {
        assert!(rows.is_empty(), "views that differ from Java's");
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
