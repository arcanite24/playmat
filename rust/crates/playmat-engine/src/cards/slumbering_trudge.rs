//! Slumbering Trudge: "This creature enters with a number of stun counters on it equal to three minus X. If X is 2 or
//! less, it enters tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SlumberingTrudge.SlumberingTrudgeCondition", x_at_most_two)];

/// `IntCompareCondition(OR_LESS, 2)` of `GetXValue`: the X it was cast with (0 when it wasn't cast)
fn x_at_most_two(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.obj(source).cast_x <= 2
}
