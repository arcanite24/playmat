//! Change of Fortune: "Discard your hand, then draw a card for each card you've discarded this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("ChangeOfFortune.ChangeOfFortuneValue", discarded)];

/// `DiscardedCardWatcher.getDiscarded(you)`
fn discarded(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.discarded.iter().filter(|s| **s == you).count() as i32
}
