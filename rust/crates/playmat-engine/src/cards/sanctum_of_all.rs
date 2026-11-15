//! Sanctum of All: "At the beginning of your upkeep, you may search your library and/or graveyard for a Shrine card
//! and put it onto the battlefield. If you search your library this way, shuffle. If an ability of another Shrine
//! you control triggers while you control six or more Shrines, that ability triggers an additional time."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "SanctumOfAll.SanctumOfAllTriggerEffect",
    &[EventType::NumberOfTriggers],
    applies,
    replace,
)];

/// the trigger is yours, you control six or more Shrines, and its source is another Shrine permanent
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let shrines = game
        .battlefield
        .iter()
        .filter(|b| game.obj(**b).controller == cx.controller && game.has_subtype(**b, "Shrine"))
        .count();
    ev.player == cx.controller
        && shrines >= 6
        && ev.source.is_some_and(|s| {
            s != cx.source && game.obj(s).zone == Zone::Battlefield && game.has_subtype(s, "Shrine")
        })
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount += 1;
    false
}
