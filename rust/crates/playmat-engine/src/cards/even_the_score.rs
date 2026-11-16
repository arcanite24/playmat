//! Even the Score: "This spell costs {U}{U}{U} less to cast if an opponent has drawn four or more cards this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("EvenTheScore.EvenTheScoreCondition", applies)];

/// `CardsDrawnThisTurnWatcher.getCardsDrawnThisTurn` of an opponent
fn applies(game: &Game, you: Seat, _: ObjId) -> bool {
    (0..game.seats()).any(|p| p != you && game.player(p).drawn >= 4)
}
