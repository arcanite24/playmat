//! Odric, Lunarch Marshal: "At the beginning of each combat, creatures you control gain first strike until end of
//! turn if a creature you control has first strike. The same is true for flying, deathtouch, double strike, haste,
//! hexproof, indestructible, lifelink, menace, reach, skulk, trample, and vigilance."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OdricLunarchMarshal.OdricLunarchMarshalEffect", apply)];

/// Java's order of keywords
const KEYWORDS: &[AbilityDef] = &[
    AbilityDef::FirstStrikeAbility,
    AbilityDef::FlyingAbility,
    AbilityDef::DeathtouchAbility,
    AbilityDef::DoubleStrikeAbility,
    AbilityDef::HasteAbility,
    AbilityDef::HexproofAbility,
    AbilityDef::IndestructibleAbility,
    AbilityDef::LifelinkAbility,
    AbilityDef::MenaceAbility,
    AbilityDef::ReachAbility,
    AbilityDef::SkulkAbility,
    AbilityDef::TrampleAbility,
    AbilityDef::VigilanceAbility,
];

/// Every check reads the battlefield as it was (Java's added effects apply only afterwards); each granted keyword
/// goes to the creatures you control now (the effect locks its objects).
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| game.obj(id).controller == you && game.has_type(id, CardType::Creature))
        .collect();
    let gained: Vec<&AbilityDef> = KEYWORDS.iter().filter(|k| mine.iter().any(|&id| game.has_ability(id, k))).collect();
    for k in gained {
        for &id in &mine {
            grant(game, id, k);
        }
    }
    Some(())
}
