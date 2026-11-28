//! Falkenrath Aristocrat: "Sacrifice a creature: Falkenrath Aristocrat gains indestructible until end of turn. If the
//! sacrificed creature was a Human, put a +1/+1 counter on Falkenrath Aristocrat."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FalkenrathAristocrat.FalkenrathAristocratEffect", apply)];

/// The creature the cost sacrificed (`SacrificeTargetCost.getPermanents().get(0)`, as it last was) was a Human, and
/// this is on the battlefield: one +1/+1 counter on it
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(&sacrificed) = cx.item.sacrificed.first() else { return Some(()) };
    let human = game.has_subtype(sacrificed, "Human");
    let me = cx.source_id();
    if human && game.obj(me).zone == Zone::Battlefield {
        add_counters_by(game, me, "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
