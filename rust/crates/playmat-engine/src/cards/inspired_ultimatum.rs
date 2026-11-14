//! Inspired Ultimatum: "Target player gains 5 life, Inspired Ultimatum deals 5 damage to any target, then you draw
//! five cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("InspiredUltimatum.InspiredUltimatumEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(p) = cx.player(0) {
        gain_life(game, p, 5);
    }
    if let Some(to) = cx.target(1) {
        deal_damage(game, cx.source_id(), to, 5, false);
    }
    draw(game, cx.controller(), 5);
    Some(())
}
