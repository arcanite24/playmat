//! Swimmer in Nightmares: "This creature gets +3/+0 as long as there are ten or more cards in a single graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("SwimmerInNightmares.SwimmerInNightmaresCondition", ten_in_one)];

/// some player in range has ten or more cards in their graveyard
fn ten_in_one(game: &Game, _: Seat, _: ObjId) -> bool {
    (0..game.seats()).any(|p| !game.player(p).lost && game.player(p).graveyard.len() >= 10)
}
