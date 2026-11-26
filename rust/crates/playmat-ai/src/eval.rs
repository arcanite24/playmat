//! How good a game is for a seat: Java's `GameStateEvaluator2` with `ArtificialScoringSystem`, the scores MAD
//! searches with, so the two AIs want the same things.

use crate::fm::Fm;
use playmat_engine::card::{AbilityDef, CardType};
use playmat_engine::state::{Game, ObjId, Seat};

pub const WIN: i64 = 100_000_000;
pub const LOSE: i64 = -WIN;

const LIFE_SCORES: [i64; 21] = [
    0, 1000, 2000, 3000, 4000, 4500, 5000, 5500, 6000, 6500, 7000, 7400, 7800, 8200, 8600, 9000, 9200, 9400, 9600,
    9800, 10000,
];
const HAND_CARD_SCORE: i64 = 5;
const PERMANENT_SCORE: i64 = 300;

/// `GameStateEvaluator2.evaluate`: the seat's life, permanents and hand against its opponent's.
pub fn evaluate(game: &Game, seat: Seat) -> i64 {
    let opp = game.opponent(seat);
    let (me, them) = (game.player(seat), game.player(opp));
    if me.lost || them.won {
        return LOSE;
    }
    if them.lost || me.won {
        return WIN;
    }
    if me.life <= 0 {
        return LOSE;
    }
    if them.life <= 0 {
        return WIN;
    }
    let mut score = life_score(me.life) - life_score(them.life);
    for id in game.battlefield.iter() {
        let o = game.obj(*id);
        let v = permanent_score(game, *id);
        if o.controller == seat {
            score += v;
        } else if o.controller == opp {
            score -= v;
        }
    }
    score + (me.hand.len() as i64 - them.hand.len() as i64) * HAND_CARD_SCORE
}

/// The score a search compares games by: version 1 is `evaluate`, 2 the fitted one (`fitted::evaluate`). Either way
/// `WIN` and `LOSE` for a game that's over.
pub fn score(game: &Game, seat: Seat, version: u8) -> i64 {
    if version >= 2 {
        crate::fitted::evaluate(game, seat)
    } else {
        evaluate(game, seat)
    }
}

/// The score a config uses (`Config::eval`): 1 Java's, 2 the fitted one, 3 the network's value (`net`)
pub fn score_with(game: &Game, seat: Seat, cfg: &crate::search::Config) -> i64 {
    if cfg.eval >= 3 {
        crate::net::score(&crate::net::of(&cfg.net), game, seat)
    } else {
        score(game, seat, cfg.eval)
    }
}

/// `value` for a config's score
pub fn value_with(game: &Game, seat: Seat, base: i64, cfg: &crate::search::Config) -> f64 {
    if cfg.eval >= 3 {
        let s = score_with(game, seat, cfg);
        return if s >= WIN {
            1.0
        } else if s <= LOSE {
            0.0
        } else {
            crate::fitted::probability(s)
        };
    }
    value(game, seat, base, cfg.eval)
}

/// A score as a value in 0..1 for averaging over samples: a win 1, a loss 0, and otherwise a squashed difference from
/// `base` (the score now) for Java's score, the predicted chance of winning for the fitted one
pub fn value(game: &Game, seat: Seat, base: i64, version: u8) -> f64 {
    let s = score(game, seat, version);
    if s >= WIN {
        return 1.0;
    }
    if s <= LOSE {
        return 0.0;
    }
    if version >= 2 {
        crate::fitted::probability(s)
    } else {
        0.5 + 0.5 * ((s - base) as f64 / 2500.0).stanh()
    }
}

/// `ArtificialScoringSystem.getLifeScore`
pub fn life_score(life: i32) -> i64 {
    let max = LIFE_SCORES.len() as i32 - 1;
    if life > max {
        LIFE_SCORES[max as usize] + (life - max) as i64 * 100
    } else if life >= 0 {
        LIFE_SCORES[life as usize]
    } else {
        0
    }
}

/// `GameStateEvaluator2.evaluatePermanent`: fixed, dynamic and combat scores
pub fn permanent_score(game: &Game, id: ObjId) -> i64 {
    let o = game.obj(id);
    let creature = game.has_type(id, CardType::Creature);
    let land = game.has_type(id, CardType::Land);
    // fixed: the card's own score, and being a permanent
    let mut score = card_score(game, id) + PERMANENT_SCORE;
    if !creature && game.has_subtype(id, "Equipment") {
        score += 100;
    }
    // dynamic
    score -= o.damage as i64 * 2;
    if creature {
        let (p, t) = game.power_toughness(id);
        let abilities: i64 = o.def.abilities.iter().map(ability_score).sum();
        score += p as i64 * 300 + t.clamp(0, 10) as i64 * 200 + abilities * (p.max(0) as i64 + 1) / 2;
    }
    // combat: what can't tap, attack or block now
    let haste = game.has_ability(id, &AbilityDef::HasteAbility);
    if o.tapped || (o.sick && creature && !haste) {
        score += if creature {
            -100
        } else if land {
            -20
        } else {
            -2
        };
    }
    if creature {
        if game.has_ability(id, &AbilityDef::DefenderAbility) {
            score -= 100;
        }
        if o.tapped {
            score -= 30;
        }
    }
    score
}

/// `ArtificialScoringSystem.getCardDefinitionScore` (every card rated 3, as in Java)
pub fn card_score(game: &Game, id: ObjId) -> i64 {
    let o = game.obj(id);
    if game.has_type(id, CardType::Land) {
        let mana = o.def.abilities.iter().filter(|a| a.mana().is_some()).count() as i64;
        return 75 + mana * 50;
    }
    let base = 300 - o.def.mana_value() as i64 * 20;
    if game.has_type(id, CardType::Creature) {
        let (p, t) = game.power_toughness(id);
        base + (p + t) as i64 * 10
    } else {
        base
    }
}

/// A card's value in hand or a library: what the AI discards last.
pub fn hand_card_score(game: &Game, id: ObjId) -> i64 {
    let o = game.obj(id);
    if o.def.types.contains(&CardType::Land) {
        return 75 + o.def.abilities.iter().filter(|a| a.mana().is_some()).count() as i64 * 50;
    }
    let base = 300 + o.def.mana_value() as i64 * 40;
    if o.def.types.contains(&CardType::Creature) {
        base + (o.def.power + o.def.toughness) as i64 * 20
    } else {
        base
    }
}

/// `MagicAbility.getAbilityScore`
fn ability_score(a: &AbilityDef) -> i64 {
    match a {
        AbilityDef::DeathtouchAbility => 60,
        AbilityDef::DefenderAbility => -100,
        AbilityDef::DoubleStrikeAbility => 100,
        AbilityDef::FirstStrikeAbility => 50,
        AbilityDef::FlashAbility => 20,
        AbilityDef::FlyingAbility => 50,
        AbilityDef::HasteAbility => 20,
        AbilityDef::IndestructibleAbility => 150,
        AbilityDef::ReachAbility => 20,
        AbilityDef::TrampleAbility => 30,
        AbilityDef::VigilanceAbility => 20,
        AbilityDef::MenaceAbility => 30,
        AbilityDef::LifelinkAbility => 40,
        AbilityDef::HexproofAbility => 60,
        _ => 2,
    }
}
