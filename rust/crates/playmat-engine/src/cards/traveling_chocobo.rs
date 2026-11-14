//! Traveling Chocobo: "If a land or Bird you control entering the battlefield causes a triggered ability of a
//! permanent you control to trigger, that ability triggers an additional time."

use super::*;
use crate::state::TriggerCause;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "TravelingChocobo.TravelingChocoboEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)];

/// The trigger is yours, a land or Bird you control entering caused it, and its source is a permanent
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(TriggerCause::Entered(id)) = game.trigger_cause else { return false };
    ev.player == cx.controller
        && game.obj(id).zone == Zone::Battlefield
        && game.obj(id).controller == cx.controller
        && (game.has_type(id, CardType::Land) || game.has_subtype(id, "Bird"))
        && ev.source.is_some_and(|s| game.obj(s).zone == Zone::Battlefield)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
