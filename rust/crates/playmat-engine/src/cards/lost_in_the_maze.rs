//! Lost in the Maze: "When Lost in the Maze enters the battlefield, tap X target creatures. Put a stun counter on each
//! of those creatures you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LostInTheMaze.LostInTheMazeEffect", apply)];

/// A stun counter on each target you don't control
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for o in cx.objs(0) {
        if game.obj(o).zone == Zone::Battlefield && game.obj(o).controller != you {
            add_counters_by(game, o, "stun", 1, Some(you));
        }
    }
    Some(())
}
