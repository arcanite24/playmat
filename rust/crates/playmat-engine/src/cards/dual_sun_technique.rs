//! Dual-Sun Technique: "Target creature you control gains double strike until end of turn. If it has a +1/+1 counter
//! on it, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DualSunTechnique.DualSunTechniqueEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).counter("+1/+1") > 0 {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
