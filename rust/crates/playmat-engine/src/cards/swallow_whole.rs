//! Swallow Whole: "As an additional cost to cast this spell, tap an untapped creature you control. Exile target
//! tapped creature. Put a +1/+1 counter on the creature tapped to pay this spell's additional cost."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SwallowWhole.SwallowWholeEffect", apply)];

/// the `TapTargetCost`'s first target, if it's still a permanent: a +1/+1 counter
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let tapped = *cx.item.tapped_for_cost.first()?;
    if game.obj(tapped).zone == Zone::Battlefield {
        add_counters_by(game, tapped, "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
