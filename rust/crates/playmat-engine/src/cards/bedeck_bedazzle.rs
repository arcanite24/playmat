//! Bedeck // Bedazzle: "Target creature gets +3/-3 until end of turn. // Destroy target nonbasic land. Bedazzle deals
//! 2 damage to target opponent or planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BedeckBedazzle.BedazzleEffect", apply)];

/// the land destroyed if it's still a legal target, then 2 damage to the second target if it still is
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(land) = cx.obj(0) {
        destroy(game, land);
    }
    if let Some(t) = cx.target(1) {
        deal_damage(game, cx.source_id(), t, 2, false);
    }
    Some(())
}
