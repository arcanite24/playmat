//! Bladecoil Serpent: "When this creature enters, for each {R}{R} spent to cast it, it gets +1/+0 and gains trample
//! and haste until end of turn." The trample and haste come only if at least one {R}{R} was spent.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BladecoilSerpent.BladecoilSerpentCondition", holds)];

/// `EachTwoManaSpentToCastValue.RED.calculate(…) > 0`
fn holds(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.obj(source).spent_colors[3] / 2 > 0
}
