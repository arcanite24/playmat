//! Fateful Handoff: "Draw cards equal to the mana value of target artifact or creature you control. An opponent
//! gains control of that permanent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FatefulHandoff.FatefulHandoffEffect", apply)];

/// The draw, then a lasting control change (`FatefulHandoffControlEffect`, for the rest of the game) to the one
/// opponent of a two-player game
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let you = cx.controller();
    draw(game, you, game.obj(o).def.mana_value() as usize);
    let opponent = (0..game.seats()).find(|p| *p != you)?;
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let before = game.obj(o).controller;
    let zcc = game.obj(o).zcc;
    game.control_changes.push(crate::state::ControlChange {
        target: o,
        zcc,
        to: opponent,
        before,
        until: crate::state::ControlUntil::Forever,
    });
    if before != opponent {
        set_controller(game, o, opponent);
    }
    refresh_control(game);
    Some(())
}
