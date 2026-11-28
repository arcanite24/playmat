//! Cosmogoyf: "This creature's power is equal to the number of cards you own in exile and its toughness is equal to
//! that number plus 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("Cosmogoyf.CosmogoyfValue", owned_in_exile)];

/// `Exile.getCardsOwned(you)`: cards, not tokens
fn owned_in_exile(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.exile.iter().filter(|c| game.obj(**c).owner == you && !game.obj(**c).token).count() as i32
}
