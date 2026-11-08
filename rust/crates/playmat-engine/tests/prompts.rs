//! Prompts and the game log (docs/rust/Views.md): replays the shadow games in `$PLAYMAT_PROMPTS` that recorded
//! them, renders the engine's prompt at each recorded question and compares it field by field with Java's, then
//! compares the engine's log lines with Java's, answer by answer, while the replay stays in step. Reports the
//! differences, most common first; `PLAYMAT_PROMPTS_SHOW=<n>` prints n examples of each; passes unless
//! `PLAYMAT_PROMPTS_STRICT=1`. Without `$PLAYMAT_PROMPTS` it checks the games checked in under `prompts/`.

use playmat_engine::decide::Query;
use playmat_engine::log::template;
use playmat_engine::prompt::prompt_view;
use playmat_engine::replay::{replay_checked, ShadowGame, ShadowPrompt};
use playmat_engine::state::Game;
use playmat_engine::view::{compare, field_of};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

type Tally = BTreeMap<String, (usize, Vec<String>)>;

fn count(t: &RefCell<Tally>, key: String, show: usize, example: String) {
    let mut m = t.borrow_mut();
    let e = m.entry(key).or_default();
    e.0 += 1;
    if e.1.len() < show {
        e.1.push(example);
    }
}

fn print(title: &str, t: Tally) {
    let mut rows: Vec<(String, (usize, Vec<String>))> = t.into_iter().collect();
    rows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(&b.0)));
    eprintln!("--- {}", title);
    for (k, (n, ex)) in rows.iter().take(60) {
        eprintln!("{:6}  {}", n, k);
        for e in ex {
            eprintln!("          {}", e);
        }
    }
}

#[test]
fn prompts_and_log_match_java() {
    let named = std::env::var("PLAYMAT_PROMPTS").ok();
    let strict = std::env::var("PLAYMAT_PROMPTS_STRICT").map(|v| v == "1").unwrap_or(false);
    let show: usize = std::env::var("PLAYMAT_PROMPTS_SHOW").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let dir = named.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("prompts"));
    let db = playmat_engine::cards();
    let mut files = Vec::new();
    collect(&dir, &mut files);
    files.sort();
    let prompt_diffs: RefCell<Tally> = RefCell::new(BTreeMap::new());
    let (mut log_missing, mut log_extra): (Tally, Tally) = (BTreeMap::new(), BTreeMap::new());
    let (prompts, same) = (RefCell::new(0usize), RefCell::new(0usize));
    let (mut lines, mut lines_same) = (0usize, 0usize);
    for f in &files {
        let shadow = ShadowGame::parse(&std::fs::read_to_string(f).unwrap()).unwrap();
        if (shadow.prompts.is_empty() && shadow.log.is_empty())
            || shadow.card_names().iter().any(|n| db.get(n).is_none())
        {
            continue;
        }
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let on_prompt = |game: &Game, q: &Query, p: &ShadowPrompt| {
            *prompts.borrow_mut() += 1;
            let at = format!("{} answer {}", name, p.decision);
            let method = p.prompt["method"].as_str().unwrap_or("?").to_string();
            let Some(rust) = prompt_view(game, p.seat, q) else {
                let msg = p.prompt["data"]["message"].as_str().unwrap_or("");
                count(&prompt_diffs, format!("unrendered {} {}", method, template(msg)), show, at);
                return;
            };
            let d = compare(&p.prompt, &rust);
            if d.is_empty() {
                *same.borrow_mut() += 1;
            }
            let mut seen = Vec::new();
            for x in &d {
                let field = format!("{} {}", method, field_of(&x.path));
                if !seen.contains(&field) {
                    seen.push(field.clone());
                    count(&prompt_diffs, field, show, format!("{} {}: Java {} | Rust {}", at, x.path, x.java, x.rust));
                }
            }
        };
        let (report, rust_log) = replay_checked(&shadow, &db, None, Some(Box::new(on_prompt)));
        if !report.complete() {
            eprintln!(
                "{}: the state differs from Java's after {} answers; what's before it compared",
                name, report.answers
            );
        }
        // the log, answer by answer, from the first answer replayed up to where the replay left Java's game
        // (what's logged before the first answer replayed happened before the game the replay starts from)
        let from = shadow.start.decision + 1;
        let upto = if report.complete() { usize::MAX } else { report.answers };
        let mut by: BTreeMap<usize, (Vec<String>, Vec<String>)> = BTreeMap::new();
        for l in shadow.log.iter().filter(|l| l.decision >= from && l.decision < upto) {
            by.entry(l.decision).or_default().0.push(l.line.clone());
        }
        for (d, l) in rust_log.iter().filter(|(d, _)| *d >= from && *d < upto) {
            by.entry(*d).or_default().1.push(l.clone());
        }
        let (mut missing, mut extra) = (RefCell::new(log_missing), RefCell::new(log_extra));
        for (d, (java, mut rust)) in by {
            for j in java {
                lines += 1;
                if let Some(i) = rust.iter().position(|r| *r == j) {
                    rust.remove(i);
                    lines_same += 1;
                } else {
                    count(&missing, template(&j), show, format!("{} answer {}: {}", name, d, j));
                }
            }
            for r in rust {
                count(&extra, template(&r), show, format!("{} answer {}: {}", name, d, r));
            }
        }
        log_missing = missing.get_mut().clone();
        log_extra = extra.get_mut().clone();
    }
    let (prompts, same) = (prompts.into_inner(), same.into_inner());
    let pd = prompt_diffs.into_inner();
    let failing = !pd.is_empty() || !log_missing.is_empty() || !log_extra.is_empty();
    print("prompt differences", pd);
    print("log lines Java wrote and the engine didn't", log_missing);
    print("log lines the engine wrote and Java didn't", log_extra);
    eprintln!("{} of {} prompts match; {} of {} log lines match", same, prompts, lines_same, lines);
    if strict {
        assert!(!failing, "prompts or log lines that differ from Java's");
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
