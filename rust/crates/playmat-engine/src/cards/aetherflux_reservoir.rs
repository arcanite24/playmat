//! Aetherflux Reservoir: "Whenever you cast a spell, you gain 1 life for each spell you've cast this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("AetherfluxReservoir.AetherfluxReservoirDynamicValue", spells_cast_this_turn)];

/// `CastSpellLastTurnWatcher.getAmountOfSpellsPlayerCastOnCurrentTurn` for the controller
fn spells_cast_this_turn(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.player(you).spells_cast as i32
}
