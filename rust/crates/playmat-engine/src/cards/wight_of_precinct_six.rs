//! Wight of Precinct Six: "This creature gets +1/+1 for each creature card in your opponents' graveyards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("WightOfPrecinctSix.CardsInOpponentGraveyardsCount", creature_cards)];

/// `CardsInOpponentGraveyardsCount(new FilterCreatureCard())`: the creature cards in your opponents' graveyards
fn creature_cards(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let them = game.opponent(you);
    game.player(them).graveyard.iter().filter(|c| game.has_type(**c, CardType::Creature)).count() as i32
}
