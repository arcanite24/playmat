//! Vnwxt, Verbose Host: "Max speed — If you would draw a card, draw two cards instead."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("VnwxtVerboseHost.VnwxtVerboseHostEffect", &[EventType::DrawCard], applies, replace)];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

/// `drawCards(2, source, game, event)`: with this replacement applied
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    draw_applied(game, cx.controller, 2, &ev.applied);
    true
}
