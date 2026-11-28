//! Wojek Investigator: "At the beginning of your upkeep, investigate once for each opponent who has more cards in hand
//! than you."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("WojekInvestigator.WojekInvestigatorValue", value)];

fn value(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mine = game.player(you).hand.len();
    (0..game.seats()).filter(|p| *p != you && game.player(*p).hand.len() > mine).count() as i32
}
