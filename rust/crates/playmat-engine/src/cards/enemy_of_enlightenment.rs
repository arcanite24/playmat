//! Enemy of Enlightenment: "This creature gets -1/-1 for each card in your opponents' hands."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("EnemyOfEnlightenment.EnemyOfEnlightenmentValue", minus_opponents_hands)];

/// minus the cards in your opponents' hands
fn minus_opponents_hands(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    -(0..game.seats()).filter(|p| *p != you).map(|p| game.player(p).hand.len() as i32).sum::<i32>()
}
