//! Liliana's Standard Bearer: "Flash. When this creature enters, draw X cards, where X is the number of creatures
//! that died under your control this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("LilianasStandardBearer.LilianasStandardBearerCount", your_creatures_died)];

/// `LilianasStandardBearerWatcher`: the creatures that died under your control this turn
fn your_creatures_died(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.died.iter().filter(|c| **c == you).count() as i32
}
