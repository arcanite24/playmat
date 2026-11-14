//! Brightspear Zealot: "This creature gets +2/+0 as long as you've cast two or more spells this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BrightspearZealot.BrightspearZealotCondition", two_spells)];

/// `SpellsCastWatcher.getSpellsCastThisTurn(you).size() >= 2`
fn two_spells(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.watch.spells.iter().filter(|s| s.caster == you).count() >= 2
}
