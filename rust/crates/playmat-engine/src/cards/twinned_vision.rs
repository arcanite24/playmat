//! Twinned Vision: "Draw a card. If this spell wasn't cast from your hand, draw two cards instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TwinnedVision.TwinnedVisionEffect", apply)];

/// One card cast from the hand, two otherwise (flashback); the spell's own zone as it was cast (`getFromZone`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = game.obj(cx.source_id());
    let from_hand = spell.zone == Zone::Stack && spell.cast_from == Some(Zone::Hand);
    draw(game, cx.controller(), if from_hand { 1 } else { 2 });
    Some(())
}
