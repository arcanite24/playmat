//! `TargetPlayerGainControlTargetPermanentEffect` (Harmless Offering): "target player gains control of target
//! permanent".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TargetPlayerGainControlTargetPermanentEffect", apply)];

/// The first target's player gains control of the second target's permanent (`GainControlTargetEffect`,
/// `Duration.Custom`: for good), as the engine's gain-control does
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let Some(o) = cx.obj(1) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield || game.player(p).lost {
        return Some(());
    }
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
