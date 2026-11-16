//! Dr. Beverly Crusher: "If you gaining life causes a triggered ability of a permanent you control to trigger, that
//! ability triggers an additional time. Whenever you gain life, put a +1/+1 counter on Dr. Crusher."

use super::*;
use crate::state::{TriggerCause, Zone};

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "DrBeverlyCrusher.DrBeverlyCrusherEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)];

/// the trigger is yours, of a permanent (`getPermanentOrLKIBattlefield`), and you gaining life caused it
/// (`GAINED_LIFE` by you)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
        && game.trigger_cause == Some(TriggerCause::GainedLife(cx.controller))
        && ev.source.is_some_and(|s| game.obj(s).zone == Zone::Battlefield)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
