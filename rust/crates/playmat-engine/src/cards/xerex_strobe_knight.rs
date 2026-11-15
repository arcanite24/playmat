//! Xerex Strobe-Knight: "{T}: Create a 2/2 white and blue Knight creature token with vigilance. Activate only if
//! you've cast two or more spells this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("XerexStrobeKnight.XerexStrobeKnightCondition", two_spells)];

/// `CastSpellLastTurnWatcher.getAmountOfSpellsPlayerCastOnCurrentTurn(you) >= 2`
fn two_spells(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.spells.iter().filter(|s| s.caster == you).count() >= 2
}
