//! `NightCondition` (Moonrager's Slash): "if it's night".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("NightCondition", night)];

fn night(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.day == Some(false)
}
