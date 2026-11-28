//! `OpponentsLostLifeCount` (Neheb, the Eternal): "the life your opponents have lost this turn".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("OpponentsLostLifeCount", value)];

fn value(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    (0..game.seats()).filter(|p| *p != you).map(|p| game.player(p).life_lost).sum()
}
