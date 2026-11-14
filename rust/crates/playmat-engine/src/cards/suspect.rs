//! Suspect (Murders at Karlov Manor): "A suspected creature has menace and can't block" (701.60). The status is a
//! mark on the permanent (`Obj::suspected`), read by `has_ability` (menace) and `can_block`; it goes with the
//! permanent when it leaves.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SuspectTargetEffect", suspect_target),
    Handler::effect("SuspectSourceEffect", suspect_source),
    Handler::predicate("SuspectedPredicate", suspected),
];

/// "suspect target creature" (`SuspectTargetEffect`): each target still on the battlefield
fn suspect_target(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for o in cx.objs(0) {
        if game.obj(o).zone == crate::state::Zone::Battlefield {
            game.obj_mut(o).set_suspected(true);
        }
    }
    Some(())
}

/// "suspect it" (`SuspectSourceEffect`): the source, if it's still the permanent it was
fn suspect_source(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if cx.source_on_battlefield(game) {
        game.obj_mut(cx.source_id()).set_suspected(true);
    }
    Some(())
}

/// `SuspectedPredicate`: a suspected permanent
fn suspected(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).zone == crate::state::Zone::Battlefield && game.obj(id).suspected()
}
