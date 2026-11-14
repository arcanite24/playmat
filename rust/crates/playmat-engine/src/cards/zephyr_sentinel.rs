//! Zephyr Sentinel: "When this creature enters, return up to one other target creature you control to its owner's
//! hand. If it was a Soldier, put a +1/+1 counter on this creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZephyrSentinel.ZephyrSentinelEffect", apply)];

/// The bounce, then the counter if it was a Soldier as it left
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    let soldier = game.has_subtype(o, "Soldier");
    game.move_to(o, Zone::Hand);
    if soldier && cx.source_on_battlefield(game) {
        add_counters_by(game, cx.source_id(), "+1/+1", 1, Some(cx.controller()));
    }
    Some(())
}
