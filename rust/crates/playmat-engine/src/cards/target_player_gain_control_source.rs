//! `TargetPlayerGainControlSourceEffect` (Humble Defector): "Target opponent gains control of Humble Defector."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TargetPlayerGainControlSourceEffect", apply)];

/// `GainControlTargetEffect(Duration.Custom, true, player)` on the source, while it's still the permanent that
/// activated: the target player controls it for the rest of the game
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let o = cx.source_id();
    let before = game.obj(o).controller;
    let zcc = game.obj(o).zcc;
    game.control_changes.push(crate::state::ControlChange {
        target: o,
        zcc,
        to: p,
        before,
        until: crate::state::ControlUntil::Forever,
    });
    if before != p {
        set_controller(game, o, p);
    }
    refresh_control(game);
    Some(())
}
