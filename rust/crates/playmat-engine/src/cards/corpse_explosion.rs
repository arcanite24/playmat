//! Corpse Explosion: "As an additional cost to cast this spell, exile a creature card from your graveyard. Corpse
//! Explosion deals damage equal to the exiled card's power to each creature and each planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CorpseExplosion.CorpseExplosionEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE_OR_PLANESWALKER"])];

/// the first card the `ExileFromGraveCost` exiled sets the damage; nothing below 1
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let power = cx.item.cost_exiled.first().map_or(0, |c| game.obj(*c).def.power);
    if power < 1 {
        return Some(());
    }
    let e = EffectDef::DamageAllEffect { amount: crate::card::Amount::Fixed(power), filter: cx.filters[0].clone() };
    cx.run(game, d, &[e]);
    Some(())
}
