//! Hazoret, Godseeker: "Hazoret can't attack or block unless you have max speed."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("HazoretGodseeker.HazoretGodseekerCondition", max_speed)];

/// `ControllerSpeedCount` is 4 or more
fn max_speed(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).speed >= 4
}
