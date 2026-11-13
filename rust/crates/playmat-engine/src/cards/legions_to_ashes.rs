//! Legions to Ashes: "Exile target nonland permanent an opponent controls and all tokens that player controls with
//! the same name as that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LegionsToAshes.LegionsToAshesEffect", apply)];

/// The tokens its controller controls with its name, and the target, exiled together
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.obj(o).controller;
    let name = game.obj(o).name().to_string();
    let mut exile: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|t| *t != o && game.obj(*t).token && game.obj(*t).controller == p && game.obj(*t).name() == name)
        .collect();
    exile.push(o);
    for t in exile {
        if game.obj(t).zone == Zone::Battlefield {
            game.move_to(t, Zone::Exile);
        }
    }
    Some(())
}
