//! Dark Matter Manipulator: "When this creature enters, mill three cards. This creature gets +2/+0 for every seven
//! cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("DarkMatterManipulator.DarkMatterManipulatorValue", two_per_seven)];

fn two_per_seven(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    (game.player(you).graveyard.len() / 7) as i32 * 2
}
