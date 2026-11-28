//! Contested Game Ball: "Whenever you're dealt combat damage, the attacking player gains control of Contested Game
//! Ball and untaps it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "ContestedGameBall.ContestedGameBallTriggeredAbility",
        "new YouDealtCombatDamageTriggeredAbility(new ContestedGameBallEffect())",
        any_batch,
    ),
    Handler::effect("ContestedGameBall.ContestedGameBallEffect", apply),
];

/// The attacking player (the active one, Java's pointer) controls it for the rest of the game, while it's still the
/// permanent it was (`GainControlTargetEffect(Duration.Custom)` on a `FixedTarget`); then it untaps, if it's still
/// there. Nothing when the ability's controller is the attacking player.
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = game.active?;
    let before = cx.controller();
    if p == before {
        return Some(());
    }
    if cx.source_on_battlefield(game) {
        let o = cx.source_id();
        let zcc = game.obj(o).zcc;
        let now = game.obj(o).controller;
        game.control_changes.push(crate::state::ControlChange {
            target: o,
            zcc,
            to: p,
            before: now,
            until: crate::state::ControlUntil::Forever,
        });
        if now != p {
            set_controller(game, o, p);
        }
        refresh_control(game);
        untap_permanent(game, o);
    }
    Some(())
}
