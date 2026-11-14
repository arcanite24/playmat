//! Razor Rings: "Razor Rings deals 4 damage to target attacking or blocking creature. You gain life equal to the
//! excess damage dealt this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RazorRings.RazorRingsEffect", apply)];

/// 4 damage with excess (`damageWithExcess`); you gain the excess
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let excess = super::goblin_negotiation::damage_with_excess(game, cx.source_id(), o, 4);
    if excess > 0 {
        gain_life(game, cx.controller(), excess);
    }
    Some(())
}
