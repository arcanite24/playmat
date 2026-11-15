//! Finneas, Ace Archer: "Whenever Finneas attacks, put a +1/+1 counter on each other creature you control that's a
//! token or a Rabbit. Then if creatures you control have total power 10 or greater, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("FinneasAceArcher.FinneasAceArcherCondition", power_ten)];

/// the total power of the creatures you control is 10 or more
fn power_ten(game: &Game, you: Seat, _: ObjId) -> bool {
    let total: i32 = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).0)
        .sum();
    total >= 10
}
