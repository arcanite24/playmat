//! Hagra Mauling: "This spell costs {1} less to cast if an opponent controls no basic lands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("HagraMauling.HagraMaulingCondition", no_basics)];

fn no_basics(game: &Game, you: Seat, _source: ObjId) -> bool {
    (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).any(|p| {
        !game.battlefield.iter().any(|b| {
            let o = game.obj(*b);
            o.controller == p && game.has_type(*b, CardType::Land) && o.def.supertypes.iter().any(|s| s == "BASIC")
        })
    })
}
