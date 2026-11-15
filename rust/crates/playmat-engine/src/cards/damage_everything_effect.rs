//! `DamageEverythingEffect(n)`: "this deals N damage to each creature and each player" (Encounter the Unknown)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DamageEverythingEffect", apply)
    .filters(&["StaticFilters.FILTER_PERMANENT_CREATURE"])
    // (only the one-argument constructor: a second argument fails "one_of:")
    .args(&["int", "one_of:"])];

/// N damage from the source to each creature, then to each player
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.arg_int(0)?;
    let src = cx.source_id();
    for o in matching(game, &cx.filters[0], cx.controller(), src) {
        deal_damage(game, src, Target::Obj(o), n, false);
    }
    for s in 0..game.seats() {
        deal_damage(game, src, Target::Player(s), n, false);
    }
    Some(())
}
