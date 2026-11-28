//! Divine Smite: "Target creature or planeswalker an opponent controls phases out. If that permanent is black, exile
//! it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DivineSmite.DivineSmiteEffect", apply)
    .effects(&["new PhaseOutTargetEffect()", "new ExileTargetEffect()"])];

/// a black target is exiled, any other phases out
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let black = game.colors(o).contains(&crate::card::Color::B);
    cx.run(game, d, &cx.effects[if black { 1 } else { 0 }..][..1]);
    (!d.stopped()).then_some(())
}
