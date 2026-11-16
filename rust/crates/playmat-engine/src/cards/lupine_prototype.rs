//! Lupine Prototype: "This creature can't attack or block unless a player has no cards in hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("LupinePrototype.LupinePrototypeCondition", empty_hand)];

/// a player in range (both, in a duel) has an empty hand
fn empty_hand(game: &Game, _: Seat, _: ObjId) -> bool {
    (0..game.seats()).any(|p| !game.player(p).lost && game.player(p).hand.is_empty())
}
