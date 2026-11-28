//! Kinbinding: "Creatures you control get +X/+X, where X is the number of creatures that entered the battlefield
//! under your control this turn. At the beginning of combat on your turn, create a 1/1 green and white Kithkin
//! creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("Kinbinding.KinbindingValue", creatures_entered)];

/// `KinbindingWatcher`: the creatures that entered under your control this turn
fn creatures_entered(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    game.watch.entered.iter().filter(|(c, _, creature, _)| *c == you && *creature).count() as i32
}
