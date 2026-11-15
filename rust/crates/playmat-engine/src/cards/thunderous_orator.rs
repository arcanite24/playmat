//! Thunderous Orator: "Vigilance. Whenever Thunderous Orator attacks, it gains flying until end of turn if you control
//! a creature with flying. The same is true for first strike, double strike, deathtouch, indestructible, lifelink,
//! menace, and trample."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThunderousOrator.ThunderousOratorEffect", apply)];

/// Menace first (a creature you control with it), then each keyword a creature you control has, in battlefield
/// order, once each: the source gains them until end of turn
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .collect();
    let mut gained: Vec<AbilityDef> = Vec::new();
    if creatures.iter().any(|o| game.has_ability(*o, &AbilityDef::MenaceAbility)) {
        gained.push(AbilityDef::MenaceAbility);
    }
    let keywords = [
        AbilityDef::FlyingAbility,
        AbilityDef::FirstStrikeAbility,
        AbilityDef::DoubleStrikeAbility,
        AbilityDef::DeathtouchAbility,
        AbilityDef::IndestructibleAbility,
        AbilityDef::LifelinkAbility,
        AbilityDef::TrampleAbility,
    ];
    for o in &creatures {
        for k in &keywords {
            if !gained.contains(k) && game.has_ability(*o, k) {
                gained.push(k.clone());
            }
        }
    }
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    for a in gained {
        grant(game, cx.source_id(), &a);
    }
    Some(())
}
