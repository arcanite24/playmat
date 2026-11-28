//! Bloodline Recollector: "At the beginning of each end step, if three or more creatures died this turn, this
//! creature becomes prepared."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("BloodlineRecollector.BloodlineRecollectorCondition", three_died)];

/// `CreaturesDiedWatcher.getAmountOfCreaturesDiedThisTurn() > 2`
fn three_died(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.watch.died.len() > 2
}
