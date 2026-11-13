//! Life's Legacy: "As an additional cost to cast this spell, sacrifice a creature. Draw cards equal to the
//! sacrificed creature's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LifesLegacy.LifesLegacyEffect", apply)];

/// The sacrificed creature's power as it was sacrificed (`sacrificed_stats`), when it's more than 0
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let power = cx.item.sacrificed_stats[0];
    if power > 0 {
        draw(game, cx.controller(), power as usize);
    }
    Some(())
}
