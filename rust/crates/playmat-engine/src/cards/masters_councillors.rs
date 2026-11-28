//! Master's Councillors: "This creature gets +2/+0 for each graveyard with seven or more cards in it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("MastersCouncillors.GraveyardsWithXOrMoreCardsInIt", graveyards_with_seven)];

/// the players in range whose graveyards have seven or more cards (the card's only `xValue`)
fn graveyards_with_seven(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    (0..game.seats()).filter(|s| game.player(*s).graveyard.len() >= 7).count() as i32
}
