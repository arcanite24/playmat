//! End-Blaze Epiphany: "End-Blaze Epiphany deals X damage to target creature. When that creature dies this turn,
//! exile a number of cards from the top of your library equal to its power, then choose a card exiled this way.
//! Until the end of your next turn, you may play that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("EndBlazeEpiphany.EndBlazeEpiphanyValue", died_power)];

/// `getValue("creatureDied")`'s power as it last was on the battlefield (none: 0)
fn died_power(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some((died, _)) = game.resolving_item.as_ref().and_then(|i| i.died) else { return 0 };
    game.obj(died).power_lki.unwrap_or(0)
}
