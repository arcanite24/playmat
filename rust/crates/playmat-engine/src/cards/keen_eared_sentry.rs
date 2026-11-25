//! Keen-Eared Sentry: "You have hexproof. Each opponent can't venture into the dungeon more than once each turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("KeenEaredSentry.KeenEaredSentryEffect", Rule::CantVenture(cant_venture))];

/// `VENTURE` by an opponent who already ventured this turn (`KeenEaredSentryWatcher`: `VENTURED`)
fn cant_venture(game: &Game, cx: &StaticCx, seat: Seat) -> bool {
    cx.opponent(seat) && game.watch.ventured.contains(&seat)
}
