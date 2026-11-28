//! The fitted score (`Config::eval` 2): a logistic model of a seat's chance of winning from a game's features, its
//! weights fitted to self-play results by `fit-eval` and kept in `fitted.json`.
//!
//! Every feature is the seat's count minus its opponent's (and "my turn" is +1 or −1), so the model is antisymmetric:
//! the two seats' chances add to 1. Each card name on the battlefield also has a learned weight: what having one
//! more copy than the opponent is worth, on top of the generic features.

use crate::eval;
use crate::fm::Fm;
use playmat_engine::card::{AbilityDef, CardType};
use playmat_engine::state::{Game, Seat};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The generic features' names, in order (`features` fills them)
pub const NAMES: [&str; 17] = [
    "my_turn",
    "life_score",
    "life",
    "creatures",
    "power",
    "toughness",
    "evasive_power",
    "lands",
    "other_permanents",
    "hand",
    "loyalty",
    "poison",
    "clock",
    "java_permanents",
    "short_library",
    "graveyard",
    "hand_mana_value",
];

/// The fitted weights
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Weights {
    pub features: Vec<f64>,
    pub cards: HashMap<String, f64>,
}

static WEIGHTS: OnceLock<Weights> = OnceLock::new();

/// The weights in `fitted.json`, read once
pub fn weights() -> &'static Weights {
    WEIGHTS.get_or_init(|| serde_json::from_str(include_str!("fitted.json")).expect("fitted.json"))
}

/// One side's raw counts
#[derive(Default, Clone, Copy)]
struct Side {
    life: f64,
    creatures: f64,
    power: f64,
    toughness: f64,
    evasive: f64,
    lands: f64,
    other: f64,
    hand: f64,
    loyalty: f64,
    poison: f64,
    java: f64,
    short_library: f64,
    graveyard: f64,
    hand_mv: f64,
}

fn side(game: &Game, seat: Seat) -> Side {
    let p = game.player(seat);
    let mut s = Side {
        life: p.life as f64,
        hand: p.hand.len() as f64,
        poison: p.counters.iter().filter(|(n, _)| n == "poison").map(|(_, v)| *v as f64).sum(),
        short_library: (p.library.len() <= 3) as u8 as f64,
        graveyard: p.graveyard.len() as f64,
        hand_mv: p.hand.iter().map(|id| game.obj(*id).def.mana_value() as f64).sum(),
        ..Side::default()
    };
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        if o.controller != seat {
            continue;
        }
        s.java += eval::permanent_score(game, *id) as f64;
        if game.has_type(*id, CardType::Creature) {
            let (pw, t) = game.power_toughness(*id);
            s.creatures += 1.0;
            if !game.has_ability(*id, &AbilityDef::DefenderAbility) {
                s.power += pw.max(0) as f64;
                if game.has_ability(*id, &AbilityDef::FlyingAbility)
                    || game.has_ability(*id, &AbilityDef::MenaceAbility)
                    || game.has_ability(*id, &AbilityDef::TrampleAbility)
                {
                    s.evasive += pw.max(0) as f64;
                }
            }
            s.toughness += t.max(0) as f64;
        } else if game.has_type(*id, CardType::Land) {
            s.lands += 1.0;
        } else {
            s.other += 1.0;
            if game.has_type(*id, CardType::Planeswalker) {
                s.loyalty += o.counter("loyalty").max(0) as f64;
            }
        }
    }
    s
}

/// The generic features from `seat`'s side, in `NAMES` order
pub fn features(game: &Game, seat: Seat) -> [f64; 17] {
    let (a, b) = (side(game, seat), side(game, game.opponent(seat)));
    // how many of the opponent's lives my creatures take a turn, capped: a race in one number
    let clock = |x: &Side, y: &Side| (x.power / y.life.max(1.0)).min(2.0);
    let turn = match game.active {
        Some(s) if s == seat => 1.0,
        Some(_) => -1.0,
        None => 0.0,
    };
    [
        turn,
        (eval::life_score(a.life as i32) - eval::life_score(b.life as i32)) as f64 / 1000.0,
        (a.life - b.life) / 10.0,
        a.creatures - b.creatures,
        (a.power - b.power) / 5.0,
        (a.toughness - b.toughness) / 5.0,
        (a.evasive - b.evasive) / 5.0,
        a.lands - b.lands,
        a.other - b.other,
        a.hand - b.hand,
        (a.loyalty - b.loyalty) / 5.0,
        (a.poison - b.poison) / 5.0,
        clock(&a, &b) - clock(&b, &a),
        (a.java - b.java) / 1000.0,
        a.short_library - b.short_library,
        (a.graveyard - b.graveyard) / 5.0,
        (a.hand_mv - b.hand_mv) / 10.0,
    ]
}

/// Each nonland card name on the battlefield, with `seat`'s copies minus its opponent's
pub fn card_counts(game: &Game, seat: Seat) -> Vec<(String, f64)> {
    let mut counts: Vec<(String, f64)> = Vec::new();
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        // lands are in the generic features: a weight per land learns which deck plays it
        if o.def.types.contains(&CardType::Land) {
            continue;
        }
        let d = if o.controller == seat { 1.0 } else { -1.0 };
        match counts.iter_mut().find(|(n, _)| *n == o.def.name) {
            Some(e) => e.1 += d,
            None => counts.push((o.def.name.clone(), d)),
        }
    }
    counts.retain(|(_, v)| *v != 0.0);
    counts
}

/// The model's logit for `seat` under `w`
pub fn logit(game: &Game, seat: Seat, w: &Weights) -> f64 {
    let f = features(game, seat);
    let mut z: f64 = f.iter().zip(&w.features).map(|(x, w)| x * w).sum();
    if !w.cards.is_empty() {
        for id in game.battlefield.iter() {
            let o = game.obj(*id);
            if let Some(v) = w.cards.get(o.def.name.as_str()) {
                z += if o.controller == seat { *v } else { -*v };
            }
        }
    }
    z
}

/// The seat's predicted chance of winning as a logit, times 1000 (`WIN`/`LOSE` for a game that's over)
pub fn evaluate(game: &Game, seat: Seat) -> i64 {
    let over = eval::evaluate(game, seat);
    if over >= eval::WIN || over <= eval::LOSE {
        return over;
    }
    (logit(game, seat, weights()) * 1000.0).clamp(-30_000.0, 30_000.0) as i64
}

/// Fitted evaluation modulated by a deck's Playbook (archetype feature multipliers, key-card bonuses, and graveyard role value).
pub fn evaluate_with_playbook(game: &Game, seat: Seat, pb: &crate::playbook::Playbook) -> i64 {
    let over = eval::evaluate(game, seat);
    if over >= eval::WIN || over <= eval::LOSE {
        return over;
    }
    let w = weights();
    let f = features(game, seat);
    let mut z: f64 = f.iter().zip(&w.features).zip(&pb.eval_weights).map(|((x, bw), mult)| x * bw * mult).sum();
    if !w.cards.is_empty() || !pb.eval_card_bonus.is_empty() {
        for id in game.battlefield.iter() {
            let o = game.obj(*id);
            let name = o.def.name.as_str();
            if let Some(v) = w.cards.get(name) {
                z += if o.controller == seat { *v } else { -*v };
            }
            if o.controller == seat {
                if let Some(bonus) = pb.eval_card_bonus.get(name) {
                    z += *bonus;
                }
            }
        }
    }
    if !pb.graveyard_role_bonus.is_empty() {
        for id in &game.player(seat).graveyard {
            let name = game.obj(*id).def.name.as_str();
            for (role, bonus) in &pb.graveyard_role_bonus {
                if pb.has_role(name, role) {
                    z += *bonus;
                }
            }
        }
    }
    (z * 1000.0).clamp(-30_000.0, 30_000.0) as i64
}

/// A fitted score as a chance of winning
pub fn probability(score: i64) -> f64 {
    1.0 / (1.0 + (-(score as f64) / 1000.0).sexp())
}
