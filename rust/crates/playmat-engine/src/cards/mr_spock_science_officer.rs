//! Mr. Spock, Science Officer: "If you would draw a card, draw two cards instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MrSpockScienceOfficer.MrSpockScienceOfficerEffect",
    &[EventType::DrawCard],
    applies,
    replace,
)];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

/// `you.drawCards(2, source, game, event)`: with this replacement applied
fn replace(game: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    draw_applied(game, ev.player, 2, &ev.applied);
    true
}
