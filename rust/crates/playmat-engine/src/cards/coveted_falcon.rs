//! Coveted Falcon: "Flying. Whenever Coveted Falcon attacks, gain control of target permanent you own but don't
//! control. Disguise {1}{U}. When Coveted Falcon is turned face up, target opponent gains control of any number of
//! target permanents you control. Draw a card for each one they gained control of this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CovetedFalcon.CovetedFalconEffect", apply)];

/// Each target permanent, in target order, goes to the target opponent for the rest of the game
/// (`GainControlTargetEffect(Duration.Custom, true, opponent)` on a `FixedTarget`); then you draw a card for each
/// one they control
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opp) = cx.player(0) else { return Some(()) };
    let permanents: Vec<ObjId> = cx.objs(1).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    for &o in &permanents {
        let zcc = game.obj(o).zcc;
        let now = game.obj(o).controller;
        game.control_changes.push(crate::state::ControlChange {
            target: o,
            zcc,
            to: opp,
            before: now,
            until: crate::state::ControlUntil::Forever,
        });
        if now != opp {
            set_controller(game, o, opp);
        }
    }
    refresh_control(game);
    let n = permanents
        .iter()
        .filter(|o| game.obj(**o).zone == Zone::Battlefield && game.obj(**o).controller == opp)
        .count();
    draw(game, cx.controller(), n);
    Some(())
}
