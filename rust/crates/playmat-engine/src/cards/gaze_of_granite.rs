//! Gaze of Granite: "Destroy each nonland permanent with mana value X or less."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GazeOfGranite.GazeOfGraniteEffect", apply)];

/// Each nonland permanent with mana value X or less, destroyed one by one in battlefield order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    let doomed: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| !game.has_type(*o, CardType::Land) && game.obj(*o).def.mana_value() as i32 <= x)
        .collect();
    for o in doomed {
        if game.obj(o).zone == Zone::Battlefield {
            destroy(game, o);
        }
    }
    Some(())
}
