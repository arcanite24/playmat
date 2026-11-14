//! Illvoi Infiltrator: "This creature can't be blocked if you've cast two or more spells this turn. Whenever this
//! creature deals combat damage to a player, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("IllvoiInfiltrator.IllvoiInfiltratorCondition", two_spells)];

/// `SpellsCastWatcher.getSpellsCastThisTurn(you).size() >= 2`
fn two_spells(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).spells_cast >= 2
}
