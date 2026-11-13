//! Emeritus of Woe: "At the beginning of your end step, if two or more creatures died this turn, this creature
//! becomes prepared."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("EmeritusOfWoe.EmeritusOfWoeCondition", two_died)];

fn two_died(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.creatures_died >= 2
}
