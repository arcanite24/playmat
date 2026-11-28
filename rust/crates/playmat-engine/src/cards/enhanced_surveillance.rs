//! Enhanced Surveillance: "You may look at an additional two cards each time you surveil."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EnhancedSurveillance.EnhancedSurveillanceReplacementEffect",
    &[EventType::Surveil],
    applies,
    replace,
)];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(2);
    false
}
