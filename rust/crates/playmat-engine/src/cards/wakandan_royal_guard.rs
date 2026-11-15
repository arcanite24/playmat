//! Wakandan Royal Guard: "When Wakandan Royal Guard enters, put a +1/+1 counter on target creature. If that creature
//! is another Hero, put two +1/+1 counters on it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WakandanRoyalGuard.WakandanRoyalGuardEffect", apply)];

/// two on another Hero, else one
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = cx.obj(0)?;
    let n = if target != cx.source_id() && game.has_subtype(target, "Hero") { 2 } else { 1 };
    add_counters_by(game, target, "+1/+1", n, Some(cx.controller()));
    Some(())
}
