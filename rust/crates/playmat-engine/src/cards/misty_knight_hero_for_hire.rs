//! Misty Knight, Hero for Hire: "{2}, {T}, Discard a card: Draw a card for each card you've discarded this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("MistyKnightHeroForHire.MistyKnightHeroForHireValue", you_discarded)];

/// `DiscardedCardWatcher.getDiscarded`: the cards you discarded this turn
fn you_discarded(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.discarded.iter().filter(|p| **p == you).count() as i32
}
