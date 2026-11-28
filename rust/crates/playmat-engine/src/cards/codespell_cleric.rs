//! Codespell Cleric: "When Codespell Cleric enters, if it was the second spell you cast this turn, put a +1/+1
//! counter on target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("CodespellCleric.CodespellClericCondition", second)];

/// `CodespellClericWatcher`: among the spells you cast this turn, in order, the first that became this permanent is
/// the second
fn second(game: &Game, you: Seat, source: ObjId) -> bool {
    game.watch.spells.iter().filter(|s| s.caster == you).position(|s| s.card == source) == Some(1)
}
