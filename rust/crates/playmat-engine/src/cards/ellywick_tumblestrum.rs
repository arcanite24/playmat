//! Ellywick Tumblestrum's emblem: "Creatures you control have trample and haste and get +2/+2 for each differently
//! named dungeon you've completed."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("EllywickTumblestrumEmblem.EllywickTumblestrumEmblemValue", per_dungeon)];

/// twice the differently named dungeons you've completed (`CompletedDungeonWatcher.getCompletedNames`)
fn per_dungeon(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut names: Vec<&String> = game.player(you).completed_dungeons.iter().collect();
    names.sort();
    names.dedup();
    2 * names.len() as i32
}
