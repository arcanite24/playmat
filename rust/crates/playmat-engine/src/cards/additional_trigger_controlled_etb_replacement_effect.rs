//! `AdditionalTriggerControlledETBReplacementEffect`: "If a permanent entering the battlefield causes a triggered
//! ability of a permanent you control to trigger, that ability triggers an additional time." (Yarok, the Desecrated;
//! Elesh Norn, Mother of Machines)

use super::*;
use crate::state::{TriggerCause, Zone};

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "AdditionalTriggerControlledETBReplacementEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    // the trigger's controller is you, a permanent entering caused it, and its source is a permanent
    ev.player == cx.controller
        && matches!(game.trigger_cause, Some(TriggerCause::Entered(_)))
        && ev.source.is_some_and(|s| game.obj(s).zone == Zone::Battlefield)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
