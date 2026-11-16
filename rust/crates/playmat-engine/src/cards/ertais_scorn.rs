//! Ertai's Scorn: "This spell costs {U} less to cast if an opponent cast two or more spells this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ErtaisScorn.ErtaisScornCondition", applies)];

/// `CastSpellLastTurnWatcher.getAmountOfSpellsPlayerCastOnCurrentTurn` of an opponent
fn applies(game: &Game, you: Seat, _: ObjId) -> bool {
    (0..game.seats()).any(|p| p != you && game.player(p).spells_cast >= 2)
}
