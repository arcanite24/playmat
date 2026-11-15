//! Blood Scrivener: "If you would draw a card while you have no cards in hand, instead you draw two cards and you
//! lose 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "BloodScrivener.BloodScrivenerReplacementEffect",
    &[EventType::DrawCard],
    applies,
    replace,
)];

/// you would draw with no cards in hand
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller && game.player(ev.player).hand.is_empty()
}

/// two cards drawn (`drawCards(2, …, event)`: this replacement already applied), then 1 life lost
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    draw_applied(game, ev.player, 2, &ev.applied);
    lose_life_by(game, ev.player, 1, Some(cx.source), false);
    true
}
