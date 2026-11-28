//! Stiltzkin, Moogle Merchant: "{2}, {T}: Target opponent gains control of another target permanent you control.
//! If they do, you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StiltzkinMoogleMerchant.StiltzkinMoogleMerchantEffect", apply)];

/// A lasting control change to the opponent (`GainControlTargetEffect(Custom, true, opponent)`), then the draw if
/// it's theirs now
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(1) else { return Some(()) };
    let Some(opponent) = cx.player(0) else { return Some(()) };
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
    if game.obj(o).controller == opponent {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
