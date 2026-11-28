//! Hapatra, the Desert Fang: "When Hapatra enters, for each opponent, put X -1/-1 counters on up to one target
//! creature that player controls, where X is the greatest mana value among cards in your graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("HapatraTheDesertFang.HapatraTheDesertFangValue", greatest)];

fn greatest(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.player(you).graveyard.iter().map(|c| game.obj(*c).def.mana_value() as i32).max().unwrap_or(0)
}
