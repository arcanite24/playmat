//! Effortless Master: "Vigilance, menace. This creature enters with two +1/+1 counters on it if you've cast two or
//! more spells this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("EffortlessMaster.EffortlessMasterCondition", two_spells)];

/// `SpellsCastWatcher.getSpellsCastThisTurn(you).size() >= 2`
fn two_spells(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).spells_cast >= 2
}
