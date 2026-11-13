//! `RegenerateAllEffect` (Golgari Charm): "Regenerate each creature you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RegenerateAllEffect", apply).args(&["filter"])];

/// A regeneration shield (`RegenerateTargetEffect` with a fixed target) on each permanent of the filter now
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let filter = cx.filters.last().cloned().unwrap_or_default();
    for o in matching(game, &filter, cx.controller(), cx.source_id()) {
        game.obj_mut(o).regen_shields += 1;
    }
    Some(())
}
