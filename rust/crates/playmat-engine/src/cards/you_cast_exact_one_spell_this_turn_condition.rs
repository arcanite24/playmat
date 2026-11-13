//! `YouCastExactOneSpellThisTurnCondition` (Highspire Bell-Ringer): "the second spell you cast each turn"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("YouCastExactOneSpellThisTurnCondition", holds)];

/// `SpellsCastWatcher.getSpellsCastThisTurn(you).size() == 1`
fn holds(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).spells_cast == 1
}
