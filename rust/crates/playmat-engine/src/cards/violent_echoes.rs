//! Violent Echoes: "Violent Echoes deals 6 damage to target creature or planeswalker. If excess damage was dealt to
//! that permanent this way, empower Jace X, where X is that excess damage."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ViolentEchoes.ViolentEchoesEffect", apply).effects(&["new EmpowerJaceEffect(1)"])];

/// `damageWithExcess`: only lethal damage is dealt (`getLethalDamage`), the rest is the excess
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let lethal = fight_lethal(game, o, cx.source_id());
    let excess = (6 - lethal).max(0);
    deal_damage(game, cx.source_id(), Target::Obj(o), 6.min(lethal), false);
    if excess > 0 {
        let mut e = cx.effects[0].clone();
        if let EffectDef::EmpowerJaceEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(excess);
        }
        cx.run(game, d, &[e]);
    }
    Some(())
}
