//! Ghoulcaller's Harvest: "Create X 2/2 black Zombie creature tokens with decayed, where X is half the number of
//! creature cards in your graveyard, rounded up."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("GhoulcallersHarvest.GhoulcallersHarvestValue", half)];

fn half(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let n = game.player(you).graveyard.iter().filter(|c| game.has_type(**c, CardType::Creature)).count() as i32;
    n / 2 + n % 2
}
