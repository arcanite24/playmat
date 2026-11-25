//! `CardsDiscardedThisTurnCount.SINGULAR` and `.PLURAL`: "a card for each card you've discarded this turn" (Change
//! of Fortune, Green Goblin, Revenant, Misty Knight, Hero for Hire), `DiscardedCardWatcher.getDiscarded(you)`. The
//! two constants differ only in their message.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("CardsDiscardedThisTurnCount.SINGULAR", you_discarded),
    Handler::value("CardsDiscardedThisTurnCount.PLURAL", you_discarded),
];

/// the cards you discarded this turn
fn you_discarded(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.discarded.iter().filter(|p| **p == you).count() as i32
}
