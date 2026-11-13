//! Hydroid Krasis: "When you cast this spell, you gain half X life and draw half X cards. Round down each time."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HydroidKrasis.HydroidKrasisEffect", apply)];

/// Half the spell's X, rounded down: you draw that many cards, then gain that much life. The cast trigger's X is the
/// spell's mana value, X included, so X is that less the printed mana value
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let printed = game.obj(cx.source_id()).def.printed().mana_value() as i32;
    let half = (cx.item.x - printed).max(0).div_euclid(2);
    draw(game, you, half as usize);
    gain_life(game, you, half);
    Some(())
}
