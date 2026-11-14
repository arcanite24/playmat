//! Search Party Captain: "This spell costs {1} less to cast for each creature you attacked with this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SearchPartyCaptain.SearchPartyCaptainValue", attackers)];

/// `PlayerAttackedWatcher.getNumberOfAttackersCurrentTurn`
fn attackers(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.player(you).attackers_this_turn as i32
}
