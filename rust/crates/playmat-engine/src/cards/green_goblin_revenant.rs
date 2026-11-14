//! Green Goblin, Revenant: "Whenever Green Goblin attacks, discard a card. Then draw a card for each card you've
//! discarded this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GreenGoblinRevenant.GreenGoblinRevenantValue", discarded)];

/// the cards you've discarded this turn (`DiscardedCardWatcher.getDiscarded`)
fn discarded(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.discarded.iter().filter(|s| **s == you).count() as i32
}
