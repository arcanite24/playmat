//! Goblin Gathering: "Create a number of 1/1 red Goblin creature tokens equal to two plus the number of cards named
//! Goblin Gathering in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GoblinGathering.GoblinGatheringDynamicValue", two_plus_named)];

fn two_plus_named(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    2 + game.player(you).graveyard.iter().filter(|c| game.obj(**c).name() == "Goblin Gathering").count() as i32
}
