//! Loan Shark: "When this creature enters, if you've cast two or more spells this turn, draw a card. Plot {3}{U}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("LoanShark.LoanSharkCondition", two_spells)];

/// `SpellsCastWatcher.getCount`: two or more spells you cast this turn
fn two_spells(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().filter(|s| s.caster == you).count() >= 2
}
