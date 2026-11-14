//! Inga Rune-Eyes: "When Inga Rune-Eyes enters the battlefield, scry 3. When Inga Rune-Eyes dies, draw three cards if
//! three or more creatures died this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("IngaRuneEyes.IngaRuneEyesCondition", three_died)];

/// `CreaturesDiedWatcher.getAmountOfCreaturesDiedThisTurn() > 2`
fn three_died(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.creatures_died > 2
}
