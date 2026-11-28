//! Asinine Antics: "You may cast Asinine Antics as though it had flash if you pay {2} more to cast it. For each
//! creature your opponents control, create a Cursed Role token attached to that creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AsinineAntics.AsinineAnticsEffect", apply)
    .effects(&["new CreateRoleAttachedTargetEffect(RoleType.CURSED)"])
    .filters(&["StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE"])];

/// `RoleType.CURSED.createToken` on each creature your opponents control, in battlefield order
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *o))
        .collect();
    for c in creatures {
        cx.run_on(game, d, &cx.effects[..1], vec![Target::Obj(c)]);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
