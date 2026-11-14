//! Beifong's Bounty Hunters: "Whenever a nonland creature you control dies, earthbend X, where X is that creature's
//! power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("BeifongsBountyHunters.BeifongsBountyHuntersValue", died_power)];

/// `getValue("creatureDied")`'s power as it last was on the battlefield (none: 0)
fn died_power(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some((died, _)) = game.resolving_item.as_ref().and_then(|i| i.died) else { return 0 };
    game.obj(died).power_lki.unwrap_or(0)
}
