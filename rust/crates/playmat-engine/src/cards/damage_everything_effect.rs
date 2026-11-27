//! `DamageEverythingEffect(n)`: "this deals N damage to each creature and each player" (Encounter the Unknown)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageEverythingEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])
    .args(&["amount", "filter"])];

/// N damage from the source to each matching creature, then to each player
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_amount(game, 0)?;
    let src = cx.source_id();
    let filter = cx.filters.get(1).unwrap_or(&cx.filters[0]);
    for o in matching(game, filter, cx.controller(), src) {
        deal_damage(game, src, Target::Obj(o), n, false);
    }
    for s in 0..game.seats() {
        deal_damage(game, src, Target::Player(s), n, false);
    }
    Some(())
}
