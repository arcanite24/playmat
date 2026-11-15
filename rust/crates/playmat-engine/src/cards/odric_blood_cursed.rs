//! Odric, Blood-Cursed: "When Odric, Blood-Cursed enters, create X Blood tokens, where X is the number of abilities
//! from among flying, first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace,
//! reach, trample, and vigilance found among creatures you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("OdricBloodCursed.OdricBloodCursedEffect", apply).tokens(&["BloodToken"])];

/// how many of the twelve keywords some creature you control has (granted ones too); that many Blood tokens
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let keywords = [
        AbilityDef::FlyingAbility,
        AbilityDef::FirstStrikeAbility,
        AbilityDef::DoubleStrikeAbility,
        AbilityDef::DeathtouchAbility,
        AbilityDef::HasteAbility,
        AbilityDef::HexproofAbility,
        AbilityDef::IndestructibleAbility,
        AbilityDef::LifelinkAbility,
        AbilityDef::MenaceAbility,
        AbilityDef::ReachAbility,
        AbilityDef::TrampleAbility,
        AbilityDef::VigilanceAbility,
    ];
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    let n = keywords.iter().filter(|k| creatures.iter().any(|c| game.has_ability(*c, k))).count() as i32;
    if n < 1 {
        return Some(());
    }
    cx.create_tokens(game, d, &cx.tokens[0], n);
    Some(())
}
