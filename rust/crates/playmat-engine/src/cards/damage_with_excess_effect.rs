//! `DamageWithExcessEffect(amount)`: "deals N damage to target creature. Excess damage is dealt to that creature's
//! controller instead" (Pigment Storm)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageWithExcessEffect", apply).args(&["int"])];

/// `Permanent.getLethalDamage`: what it takes to destroy it (a creature's toughness less its damage, 1 from a
/// deathtouch source; a planeswalker's loyalty; a battle's defense), the least of those it is
pub(crate) fn lethal_from(game: &Game, id: ObjId, source: ObjId) -> i32 {
    let mut lethal = i32::MAX;
    if game.has_type(id, CardType::Creature) {
        let (_, t) = game.power_toughness(id);
        lethal = (t - game.obj(id).damage).max(0);
        if game.has_ability(source, &AbilityDef::DeathtouchAbility) {
            lethal = lethal.min(1);
        }
    }
    if game.has_type(id, CardType::Planeswalker) {
        lethal = lethal.min(game.obj(id).counter("loyalty"));
    }
    if game.has_type(id, CardType::Battle) {
        lethal = lethal.min(game.obj(id).counter("defense"));
    }
    lethal
}

/// `Permanent.damageWithExcess`: up to lethal damage to the permanent; returns the rest
pub(crate) fn damage_with_excess(game: &mut Game, source: ObjId, id: ObjId, amount: i32) -> i32 {
    let lethal = lethal_from(game, id, source);
    let excess = (amount - lethal).max(0);
    deal_damage(game, source, Target::Obj(id), amount.min(lethal), false);
    excess
}

/// The excess goes to the creature's controller (as it is after the damage)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return None;
    }
    let amount = cx.arg_int(0)?;
    let excess = damage_with_excess(game, cx.source_id(), o, amount);
    if excess > 0 {
        let p = game.obj(o).controller;
        deal_damage(game, cx.source_id(), Target::Player(p), excess, false);
    }
    Some(())
}
