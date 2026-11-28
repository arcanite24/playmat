//! Hungering Hydra: "Whenever this creature is dealt damage, put that many +1/+1 counters on it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HungeringHydra.HungeringHydraEffect", apply)];

/// the damage is the trigger's X; `AddCountersSourceEffect` on the permanent it is now
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let n = cx.item.x;
    let o = cx.source_id();
    if n > 0 && game.obj(o).zone == Zone::Battlefield {
        add_counters_by(game, o, "+1/+1", n, Some(cx.controller()));
    }
    Some(())
}
