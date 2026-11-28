//! Wishclaw Talisman: "{1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it
//! into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WishclawTalisman.WishclawTalismanEffect", apply)];

/// An opponent (`TargetOpponent`, not targeted: in a duel the one opponent, auto-chosen) gains control of the
/// Talisman for the rest of the game (`GainControlTargetEffect(Duration.Custom, true, …)` on a `FixedTarget`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(to) = (0..game.seats()).map(|i| (you + i) % game.seats()).find(|p| *p != you && !game.player(*p).lost)
    else {
        return Some(());
    };
    let o = cx.source_id();
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let before = game.obj(o).controller;
    let zcc = game.obj(o).zcc;
    game.control_changes.push(crate::state::ControlChange {
        target: o,
        zcc,
        to,
        before,
        until: crate::state::ControlUntil::Forever,
    });
    if before != to {
        set_controller(game, o, to);
    }
    refresh_control(game);
    Some(())
}
