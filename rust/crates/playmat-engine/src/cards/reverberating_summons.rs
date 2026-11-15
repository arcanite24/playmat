//! Reverberating Summons: "At the beginning of each combat, if you've cast two or more spells this turn, this
//! enchantment becomes a 3/3 Monk creature with haste until end of turn in addition to its other types. {1}{R},
//! Discard your hand, Sacrifice this enchantment: Draw two cards."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("ReverberatingSummons.ReverberatingSummonsCondition", two_spells)];

/// `SpellsCastWatcher.getCount(you) >= 2`
fn two_spells(game: &Game, you: Seat, _: ObjId) -> bool {
    game.watch.spells.iter().filter(|s| s.caster == you).count() >= 2
}
