//! How good a game is for a seat: Java's `GameStateEvaluator2` with `ArtificialScoringSystem`, the scores MAD
//! searches with, so the two AIs want the same things.

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
