//! Raze to the Ground: "This spell can't be countered. Destroy target artifact. If its mana value was 1 or less, draw
//! a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RazeToTheGround.RazeToTheGroundEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let mv = game.obj(o).def.mana_value();
    destroy(game, o);
    if mv <= 1 {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
