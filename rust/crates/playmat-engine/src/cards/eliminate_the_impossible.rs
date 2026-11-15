//! Eliminate the Impossible: "Investigate. Creatures your opponents control get -2/-0 until end of turn. If any of
//! them are suspected, they're no longer suspected."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("EliminateTheImpossible.EliminateTheImpossibleEffect", apply)];

/// each suspected creature an opponent controls stops being suspected
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let suspects: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| {
            game.obj(*o).controller != you && game.has_type(*o, CardType::Creature) && game.obj(*o).suspected()
        })
        .collect();
    for o in suspects {
        game.obj_mut(o).set_suspected(false);
    }
    Some(())
}
