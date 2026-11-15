//! Tormented Thoughts: "As an additional cost to cast this spell, sacrifice a creature. Target player discards a
//! number of cards equal to that creature's power."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TormentedThoughts.TormentedThoughtsDiscardEffect", apply)];

/// the sacrificed creature's power as it last was on the battlefield; that many discarded by the target player
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    // (`SacrificeCostCreaturesPower`: the cost's one creature)
    let power = cx.item.sacrificed_stats[0];
    if power > 0 {
        discard(game, d, p, power);
    }
    (!d.stopped()).then_some(())
}
