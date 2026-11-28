//! The tactics suite (archbench/tactics-1): positions with a known right line. Each puzzle sets up a game at a
//! step of a turn, the entrant plays both seats from there, and a goal checks the result: seat 0 won by a turn, is
//! still alive when a turn begins, or got rid of a card. Each puzzle is played with a few seeds (the libraries, and the
//! AI's samples), and an entrant's score is the share solved.
//!
//! cargo run --release -p playmat-ai --bin tactics -- [--suite archbench/tactics-1]
//!     [--entrants rust-playouts@s4,host@s4] [--seeds 4] [--only name,name] [--verbose]

use playmat_ai::entrants::{entrant, Entrant};
use playmat_ai::search::Table;
use playmat_engine::card::{CardDb, CardType};
use playmat_engine::rng::JavaRandom;
use playmat_engine::rules;
use playmat_engine::state::{Game, Seat, Step, Zone};
use serde_json::Value;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let repo = env!("CARGO_MANIFEST_DIR").to_string() + "/../../..";
    let dir = opt("--suite").unwrap_or_else(|| format!("{repo}/archbench/tactics-1"));
    let suite: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/puzzles.json")).expect("puzzles.json"))
            .expect("puzzles.json");
    let names: Vec<String> = opt("--entrants")
        .unwrap_or_else(|| "rust-quick,rust-playouts@s4,host@s4".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let entrants: Vec<Entrant> = names.iter().map(|n| entrant(n).unwrap_or_else(|e| panic!("{e}"))).collect();
    let seeds: u64 = opt("--seeds").and_then(|v| v.parse().ok()).unwrap_or(4);
    let only: Option<Vec<String>> = opt("--only").map(|v| v.split(',').map(|s| s.to_string()).collect());
    let verbose = args.iter().any(|a| a == "--verbose");
    let db = playmat_engine::cards();
    let puzzles: Vec<&Value> = suite["puzzles"]
        .as_array()
        .expect("puzzles")
        .iter()
        .filter(|p| only.as_ref().is_none_or(|o| o.iter().any(|n| p["name"].as_str() == Some(n))))
        .collect();
    println!("TACTICS {} puzzles, {} seeds each, entrants {}", puzzles.len(), seeds, names.join(", "));
    let mut totals = vec![(0u32, 0u32); entrants.len()];
    for p in &puzzles {
        let name = p["name"].as_str().unwrap_or("?");
        let mut line = format!("TACTICS {name:<32}");
        for (e, ent) in entrants.iter().enumerate() {
            let mut solved = 0;
            for seed in 0..seeds {
                let ok = match setup(&db, p, seed) {
                    Ok(game) => solve(game, p, ent, verbose),
                    Err(err) => panic!("puzzle {name}: {err}"),
                };
                solved += ok as u32;
            }
            totals[e].0 += solved;
            totals[e].1 += seeds as u32;
            line += &format!("  {}/{}", solved, seeds);
        }
        println!("{line}");
    }
    for (e, ent) in entrants.iter().enumerate() {
        let (s, n) = totals[e];
        println!("TACTICS {:<24} solved {s}/{n} ({:.0}%)", ent.name, 100.0 * s as f64 / n.max(1) as f64);
    }
}

/// The puzzle's game: each seat's zones, life and the step seat `active` (default 0) has priority in
fn setup(db: &CardDb, p: &Value, seed: u64) -> Result<Game, String> {
    let mut game = Game::new(2, JavaRandom::new(seed as i64 * 7919 + 17).state());
    let seats = p["seats"].as_array().ok_or("seats")?;
    for (seat, s) in seats.iter().enumerate() {
        game.player_mut(seat).life = s["life"].as_i64().unwrap_or(20) as i32;
        for (zone, key) in [
            (Zone::Battlefield, "battlefield"),
            (Zone::Hand, "hand"),
            (Zone::Graveyard, "graveyard"),
            (Zone::Library, "library"),
        ] {
            let empty = Vec::new();
            let cards = s[key].as_array().unwrap_or(&empty);
            let mut list: Vec<Value> = cards.clone();
            if zone == Zone::Library && list.is_empty() {
                // a library of the seat's first basic land, so nobody decks
                let basic = seats[seat]["battlefield"]
                    .as_array()
                    .and_then(|b| {
                        b.iter()
                            .map(card_name)
                            .find(|n| ["Plains", "Island", "Swamp", "Mountain", "Forest"].contains(n))
                    })
                    .unwrap_or("Plains")
                    .to_string();
                list = (0..20).map(|_| Value::String(basic.clone())).collect();
            }
            for c in &list {
                let name = card_name(c);
                let count = c["count"].as_u64().unwrap_or(1);
                for _ in 0..count {
                    let def = db.get(name).ok_or_else(|| format!("card not in the engine: {name}"))?;
                    let id = game.create_obj(Arc::clone(&def), seat, zone);
                    match zone {
                        Zone::Battlefield => {
                            Arc::make_mut(&mut game.battlefield).push(id);
                            let o = game.obj_mut(id);
                            o.tapped = c["tapped"].as_bool().unwrap_or(false);
                            o.sick = c["sick"].as_bool().unwrap_or(false);
                            o.damage = c["damage"].as_i64().unwrap_or(0) as i32;
                            o.entered_turn = 0;
                            if let Some(counters) = c["counters"].as_object() {
                                for (k, v) in counters {
                                    o.add_counters(k, v.as_i64().unwrap_or(0) as i32);
                                }
                            }
                            if def.types.contains(&CardType::Planeswalker) && o.counter("loyalty") == 0 {
                                o.add_counters("loyalty", def.loyalty.unwrap_or(0));
                            }
                        }
                        Zone::Hand => game.player_mut(seat).hand.push(id),
                        Zone::Graveyard => game.player_mut(seat).graveyard.push(id),
                        _ => game.player_mut(seat).library.push(id),
                    }
                }
            }
        }
        if zone_shuffle(p) {
            let mut library = game.player(seat).library.clone();
            game.rng.shuffle(&mut library);
            game.player_mut(seat).library = library;
        }
    }
    let active = p["active"].as_u64().unwrap_or(0) as Seat;
    game.turn = p["turn"].as_u64().unwrap_or(5) as u32;
    game.step = Some(Step::parse(p["step"].as_str().unwrap_or("PRECOMBAT_MAIN")).ok_or("step")?);
    game.active = Some(active);
    game.priority = Some(active);
    Ok(game)
}

fn zone_shuffle(p: &Value) -> bool {
    p["shuffle"].as_bool().unwrap_or(true)
}

fn card_name(c: &Value) -> &str {
    c.as_str().or(c["name"].as_str()).unwrap_or("?")
}

/// Plays the puzzle out with the entrant on both seats and checks its goal
fn solve(mut game: Game, p: &Value, ent: &Entrant, verbose: bool) -> bool {
    let goal = &p["goal"];
    let turn = game.turn;
    let by = turn + goal["turns"].as_u64().unwrap_or(0) as u32;
    let mut table = Table::new(vec![ent.config.clone(), ent.config.clone()]);
    let run = catch_unwind(AssertUnwindSafe(|| rules::play_from_priority(&mut game, &mut table, by + 1)));
    if run.is_err() {
        if verbose {
            println!("  {} {}: engine panic", p["name"], ent.name);
        }
        return false;
    }
    let me: Seat = 0;
    let opp: Seat = 1;
    let won = game.player(opp).lost || game.player(me).won || game.player(opp).life <= 0;
    let lost = game.player(me).lost || game.player(me).life <= 0;
    let ok = match goal["kind"].as_str().unwrap_or("win") {
        // seat 0 wins before turn `turns` after this one ends
        "win" => won && !lost,
        // seat 0 is alive when the turn after `turns` more begins
        "survive" => !lost,
        // no permanent named `card` controlled by seat 1 is left, and seat 0 is alive
        "remove" => {
            let card = goal["card"].as_str().unwrap_or("");
            !lost
                && !game.battlefield.iter().any(|id| game.obj(*id).controller == opp && game.obj(*id).def.name == card)
        }
        // seat 0 still controls a permanent named `card`, and is alive
        "keep" => {
            let card = goal["card"].as_str().unwrap_or("");
            !lost && game.battlefield.iter().any(|id| game.obj(*id).controller == me && game.obj(*id).def.name == card)
        }
        // seat 0 has at least `life` life, and is alive
        "life" => !lost && game.player(me).life >= goal["life"].as_i64().unwrap_or(1) as i32,
        k => panic!("unknown goal {k}"),
    };
    if verbose {
        println!(
            "  {} {}: {} (turn {}, life {} to {})",
            p["name"],
            ent.name,
            if ok { "solved" } else { "missed" },
            game.turn,
            game.player(me).life,
            game.player(opp).life
        );
    }
    ok
}
