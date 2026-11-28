//! How the AI does against people: reads hosted games' records (docs/rust/GameHost.md, "Game records") and counts
//! the AI seat's results by skill, by the AI's config, by the AI's deck and by the person's deck.
//!
//!   cargo run --release -p playmat-host --bin host-results -- <records folder or files…> [--since 2026-11-14]
//!
//! Only one-on-one games of a person against the AI count. A deck is named after the Career school deck it matches
//! card for card (Mage.Server/career-src), else "other". `--since` keeps records modified on or after a date.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default, Clone, Copy)]
struct Tally {
    ai_won: u32,
    drew: u32,
    ai_lost: u32,
    conceded: u32,
    turns: u32,
}

impl Tally {
    fn add(&mut self, ai_won: Option<bool>, conceded: bool, turns: u32) {
        match ai_won {
            Some(true) => self.ai_won += 1,
            Some(false) => self.ai_lost += 1,
            None => self.drew += 1,
        }
        self.conceded += conceded as u32;
        self.turns += turns;
    }
    fn games(&self) -> u32 {
        self.ai_won + self.drew + self.ai_lost
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let since = args.iter().position(|a| a == "--since").and_then(|i| args.get(i + 1)).cloned();
    let mut files = Vec::new();
    let mut skip = false;
    for a in &args {
        if skip {
            skip = false;
            continue;
        }
        if a == "--since" {
            skip = true;
            continue;
        }
        collect(Path::new(a), &mut files);
    }
    if files.is_empty() {
        eprintln!("usage: host-results <records folder or files…> [--since YYYY-MM-DD]");
        std::process::exit(2);
    }
    let since_secs = since.as_deref().and_then(date_secs);
    let schools = school_decks();
    let mut by: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let (mut counted, mut skipped) = (0, 0);
    for f in &files {
        if let (Some(s), Ok(meta)) = (since_secs, std::fs::metadata(f)) {
            let modified = meta.modified().ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok());
            if modified.is_some_and(|m| m.as_secs() < s) {
                continue;
            }
        }
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let mut start = None;
        let mut end = None;
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
            if v.get("start").is_some() {
                start = Some(v["start"].clone());
            } else if v.get("end").is_some() {
                end = Some(v["end"].clone());
            }
        }
        let (Some(start), Some(end)) = (start, end) else {
            skipped += 1;
            continue;
        };
        let seats = start["seats"].as_array().cloned().unwrap_or_default();
        let humans: Vec<usize> = (0..seats.len()).filter(|i| seats[*i]["human"].as_bool() == Some(true)).collect();
        if seats.len() != 2 || humans.len() != 1 {
            skipped += 1;
            continue;
        }
        let (human, ai) = (humans[0], 1 - humans[0]);
        let ai_won = end["winner"].as_u64().map(|w| w as usize == ai);
        let reason = end["reason"].as_str().unwrap_or("");
        let conceded = reason.contains("conced") || reason.contains("left") || reason.contains("quit");
        let turns = end["turns"].as_u64().unwrap_or(0) as u32;
        let skill = start["rules"]["aiSkill"].as_u64().map(|s| s.to_string()).unwrap_or_else(|| "default".into());
        let config = start["ai"].as_str().unwrap_or("(before configs were recorded)").to_string();
        let deck = |s: usize| deck_name(&seats[s]["deck"], &schools);
        for key in [
            ("all".to_string(), String::new()),
            ("skill".to_string(), skill),
            ("ai config".to_string(), config),
            ("ai deck".to_string(), deck(ai)),
            ("person's deck".to_string(), deck(human)),
            ("reason".to_string(), reason.to_string()),
        ] {
            by.entry(key).or_default().add(ai_won, conceded, turns);
        }
        counted += 1;
    }
    println!("HOST RESULTS {counted} games of a person against the AI ({skipped} records skipped)");
    let mut group = String::new();
    for ((g, k), t) in &by {
        if *g != group {
            println!("\n{g}");
            group = g.clone();
        }
        let n = t.games().max(1) as f64;
        println!(
            "  {:<48} {:>4} games  AI {:>3}-{}-{:<3} ({:>3.0}%)  conceded {:>3}  {:.1} turns",
            if k.is_empty() { "(every game)" } else { k },
            t.games(),
            t.ai_won,
            t.drew,
            t.ai_lost,
            100.0 * (t.ai_won as f64 + t.drew as f64 / 2.0) / n,
            t.conceded,
            t.turns as f64 / n
        );
    }
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(rd) = std::fs::read_dir(path) {
            let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            entries.sort();
            for e in entries {
                collect(&e, out);
            }
        }
    } else if path.extension().is_some_and(|e| e == "jsonl") {
        out.push(path.to_path_buf());
    }
}

/// The Career school decks, each as its sorted card list
fn school_decks() -> Vec<(String, Vec<String>)> {
    let root = env!("CARGO_MANIFEST_DIR").to_string() + "/../../../Mage.Server/career-src";
    let mut out = Vec::new();
    for dir in ["pioneer", "standard", "pauper", "modern"] {
        let Ok(rd) = std::fs::read_dir(format!("{root}/{dir}")) else { continue };
        for f in rd.flatten().map(|e| e.path()).filter(|f| f.extension().is_some_and(|e| e == "txt")) {
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let deck = playmat_ai::duel::Deck::parse(f.file_stem().unwrap().to_str().unwrap(), &text);
            let mut cards = deck.cards.clone();
            cards.sort();
            out.push((format!("{dir}/{}", deck.name), cards));
        }
    }
    out
}

fn deck_name(deck: &Value, schools: &[(String, Vec<String>)]) -> String {
    let mut cards: Vec<String> =
        deck.as_array().map(|d| d.iter().filter_map(|c| c.as_str().map(str::to_string)).collect()).unwrap_or_default();
    cards.sort();
    if let Some((n, _)) = schools.iter().find(|(_, c)| *c == cards) {
        return n.clone();
    }
    // another deck: by its most played card that isn't a basic land
    let basics = ["Plains", "Island", "Swamp", "Mountain", "Forest", "Wastes"];
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for c in cards.iter().filter(|c| !basics.contains(&c.as_str())) {
        *counts.entry(c.as_str()).or_default() += 1;
    }
    let top = counts.into_iter().max_by_key(|(n, c)| (*c, std::cmp::Reverse(*n))).map(|(n, _)| n.to_string());
    format!("other ({})", top.unwrap_or_default())
}

/// Seconds since 1970 at the start of a `YYYY-MM-DD` day (UTC)
fn date_secs(d: &str) -> Option<u64> {
    let mut it = d.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (it.next()??, it.next()??, it.next()??);
    // days from civil (Howard Hinnant)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    (days >= 0).then_some(days as u64 * 86_400)
}
