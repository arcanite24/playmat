//! One Archbench deal between two entrants, with the game log: to read how an AI plays.
//!
//! cargo run --release -p playmat-ai --bin ai-game -- --deal 17 --a host@s4 --b random [--suite archbench/constructed-2]
//!     [--turns 30] [--find "Greenhorn Stompy" --winner b]
//!
//! `--find` plays deals from `--deal` on until one where the named deck (seat A's or B's) is on the `--winner` side,
//! and prints that one.

use playmat_ai::duel::{new_duel, Deck};
use playmat_ai::entrants::entrant;
use playmat_ai::mulligan;
use playmat_ai::search::Table;
use playmat_engine::rules;
use serde_json::Value;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let repo = env!("CARGO_MANIFEST_DIR").to_string() + "/../../..";
    let suite_dir = opt("--suite").unwrap_or_else(|| format!("{repo}/archbench/constructed-2"));
    let suite: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{suite_dir}/suite.json")).unwrap()).unwrap();
    let decks: Vec<Deck> = suite["decks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let path = d["file"].as_str().unwrap();
            let text = std::fs::read_to_string(format!("{repo}/{path}")).unwrap();
            Deck::parse(std::path::Path::new(path).file_stem().unwrap().to_str().unwrap(), &text)
        })
        .collect();
    let n = decks.len();
    let a = entrant(&opt("--a").unwrap_or_else(|| "host@s4".into())).unwrap();
    let b = entrant(&opt("--b").unwrap_or_else(|| "random".into())).unwrap();
    let turns: u32 = opt("--turns").and_then(|v| v.parse().ok()).unwrap_or(30);
    let first: usize = opt("--deal").and_then(|v| v.parse().ok()).unwrap_or(0);
    let find = opt("--find");
    let want_b = opt("--winner").as_deref() == Some("b");
    let db = playmat_engine::cards();
    for deal in first..first + if find.is_some() { 5000 } else { 1 } {
        let da = deal % n;
        let dbk = (da + 1 + (deal / n) % (n - 1)) % n;
        let pair = [&decks[da], &decks[dbk]];
        if let Some(name) = &find {
            if !pair.iter().any(|d| &d.name == name) {
                continue;
            }
        }
        let mut game = new_duel(&db, pair, deal as i64, deal % 2).unwrap();
        game.log = Some(Vec::new());
        let configs = [a.config.clone(), b.config.clone()];
        if suite["mulligans"].as_bool().unwrap_or(false) {
            mulligan::london(
                &mut game,
                deal % 2,
                0,
                &mut |g, s, t| mulligan::wants(g, s, t, configs[s].as_ref()),
                &mut |g, s| mulligan::bottom(g, s, configs[s].as_ref()),
            );
        }
        let mut table = Table::new(configs.to_vec());
        rules::play_from_priority(&mut game, &mut table, turns);
        let winner = if game.player(0).lost {
            Some(1)
        } else if game.player(1).lost {
            Some(0)
        } else {
            None
        };
        if let Some(name) = &find {
            let side = if want_b { 1 } else { 0 };
            let deck_side = pair.iter().position(|d| &d.name == name).unwrap();
            if winner != Some(side) || deck_side != side {
                continue;
            }
        }
        println!("deal {deal}: A {} ({}) against B {} ({})", a.name, pair[0].name, b.name, pair[1].name);
        for line in game.log.as_ref().unwrap() {
            println!("{}", strip(line));
        }
        println!(
            "winner: {:?} at turn {}, life A {} B {}",
            winner,
            game.turn,
            game.player(0).life,
            game.player(1).life
        );
        return;
    }
    println!("no such deal");
}

/// The log line without its HTML
fn strip(s: &str) -> String {
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
