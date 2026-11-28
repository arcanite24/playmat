//! Akroan Horse: "When Akroan Horse enters, an opponent gains control of it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AkroanHorse.AkroanHorseChangeControlEffect", apply)];

/// You choose an opponent (`TargetOpponent`, not targeted), who controls this permanent for the rest of the game
/// while it's the permanent that entered (`AkroanHorseGainControlEffect`, `Duration.Custom`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let chosen = choose_target(game, d, you, &crate::card::TargetDef::TargetOpponent, Some(cx.source_id()), &rule, false);
    if d.stopped() {
        return None;
    }
    let Some(Target::Player(p)) = chosen.and_then(|c| c.first().copied()) else { return Some(()) };
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
