//! Cut Propulsion: "Target creature deals damage to itself equal to its power. If that creature has flying, it deals
//! twice that much damage to itself instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CutPropulsion.CutPropulsionEffect", apply)];

/// the creature is the damage's source
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let times = if game.has_ability(o, &AbilityDef::FlyingAbility) { 2 } else { 1 };
    let damage = times * game.power_toughness(o).0;
    if damage > 0 {
        deal_damage(game, o, Target::Obj(o), damage, false);
    }
    Some(())
}
