//! Narset, Jeskai Waymaster: "At the beginning of your end step, you may discard your hand. If you do, draw a card
//! for each spell you've cast this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("NarsetJeskaiWaymaster.NarsetJeskaiWaymasterValue", spells_cast_this_turn)];

/// `CastSpellLastTurnWatcher.getAmountOfSpellsPlayerCastOnCurrentTurn(you)`
fn spells_cast_this_turn(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.spells.iter().filter(|s| s.caster == you).count() as i32
}
