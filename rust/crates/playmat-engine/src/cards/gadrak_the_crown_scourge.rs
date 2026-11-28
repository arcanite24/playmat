//! Gadrak, the Crown-Scourge: "At the beginning of your end step, create a Treasure token for each nontoken creature
//! that died this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GadrakTheCrownScourge.GadrakTheCrownScourgeValue", nontoken_died)];

/// `GadrakTheCrownScourgeWatcher`: the nontoken creatures that died this turn, anyone's
fn nontoken_died(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.dies.iter().filter(|(_, _, creature, token)| *creature && !*token).count() as i32
}
