//! Hoard Hauler: "Whenever Hoard Hauler deals combat damage to a player, create a Treasure token for each artifact
//! that player controls."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::value("HoardHauler.HoardHaulerValue", artifacts_they_control)];

/// the artifacts the damaged player (the trigger's player pointer: in a duel, your opponent) controls
fn artifacts_they_control(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let them = game.opponent(you);
    game.battlefield
        .iter()
        .filter(|o| {
            let p = game.obj(**o);
            p.zone == Zone::Battlefield && p.controller == them && game.has_type(**o, CardType::Artifact)
        })
        .count() as i32
}
