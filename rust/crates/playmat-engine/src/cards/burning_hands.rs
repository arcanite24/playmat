//! Burning Hands: "Burning Hands deals 2 damage to target creature or planeswalker. If that permanent is green,
//! Burning Hands deals 6 damage instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BurningHands.BurningHandsEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let n = if game.colors(o).contains(&Color::G) { 6 } else { 2 };
    deal_damage(game, cx.source_id(), Target::Obj(o), n, false);
    Some(())
}
