//! Essence Flux: "Exile target creature you control, then return that card to the battlefield under its owner's
//! control. If it's a Spirit, put a +1/+1 counter on it." (`ExileThenReturnTargetEffect(false, false)` with an
//! `afterEffect`, pointed at what came back)

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::effect("EssenceFlux.ExileThenReturnTargetEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let back = flicker(game, d, cx, None);
    // `ConditionalOneShotEffect(AddCountersTargetEffect, TargetHasSubtypeCondition(SPIRIT))`: the first target as it
    // is now
    if let Some(&o) = back.first() {
        if game.obj(o).zone == Zone::Battlefield && game.has_subtype(o, "Spirit") {
            add_counters_by(game, o, "+1/+1", 1, Some(cx.controller()));
        }
    }
    Some(())
}

/// `ExileThenReturnTargetEffect(false, false)`: the targets on the battlefield exiled together, then each card
/// returned under its owner's control in turn (`withEnterWithCounters`: with these counters); what came back
pub(super) fn flicker(
    game: &mut Game,
    d: &mut dyn Decider,
    cx: &EffectCx,
    counters: Option<(&str, i32)>,
) -> Vec<ObjId> {
    let gone: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    for o in &gone {
        game.move_to(*o, Zone::Exile);
    }
    let mut back = Vec::new();
    for o in gone {
        // (a token ceases to exist in exile)
        if game.obj(o).zone == Zone::Exile && !game.obj(o).token {
            let owner = game.obj(o).owner;
            enter_battlefield(game, d, o, owner, false);
            if game.obj(o).zone == Zone::Battlefield {
                if let Some((name, n)) = counters {
                    add_counters_by(game, o, name, n, Some(cx.controller()));
                }
                back.push(o);
            }
        }
    }
    back
}
