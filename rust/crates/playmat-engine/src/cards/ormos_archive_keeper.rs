//! Ormos, Archive Keeper: "If you would draw a card while your library has no cards in it, instead put five +1/+1
//! counters on Ormos, Archive Keeper. {1}{U}{U}, Discard three cards with different names: Draw five cards." (The
//! cost is `DiscardCardCost`'s `different_names`.)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "OrmosArchiveKeeper.OrmosArchiveKeeperEffect",
    &[EventType::DrawCard],
    applies,
    counters,
)];

/// your draw while your library is empty (and you haven't lost)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller && game.player(ev.player).library.is_empty() && !game.player(ev.player).lost
}

/// five +1/+1 counters on this instead, while it's on the battlefield
fn counters(game: &mut Game, cx: &StaticCx, _: &mut Event) -> bool {
    if game.obj(cx.source).zone == Zone::Battlefield {
        add_counters_by(game, cx.source, "+1/+1", 5, Some(cx.controller));
    }
    true
}
