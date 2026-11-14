//! Akoum Flameseeker: "Cohort — {T}, Tap an untapped Ally you control: Discard a card. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AkoumFlameseeker.AkoumFlameseekerEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    if !discarded.is_empty() {
        draw(game, you, 1);
    }
    Some(())
}
