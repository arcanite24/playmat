//! Vren, the Relentless: "At the beginning of each end step, create X 1/1 black Rat creature tokens with "This
//! creature gets +1/+1 for each other Rat you control," where X is the number of creatures your opponents controlled
//! that were exiled this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("VrenTheRelentless.VrenTheRelentlessCount", exiled_of_opponents)];

/// `VrenTheRelentlessWatcher`: the creatures exiled from the battlefield this turn whose controller then was an
/// opponent of yours
fn exiled_of_opponents(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.exiled_creatures.iter().filter(|c| **c != you).count() as i32
}
