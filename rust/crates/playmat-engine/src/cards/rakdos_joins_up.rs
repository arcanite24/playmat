//! Rakdos Joins Up: "When Rakdos Joins Up enters, return target creature card from your graveyard to the battlefield
//! with two additional +1/+1 counters on it. Whenever a legendary creature you control dies, Rakdos Joins Up deals
//! damage equal to that creature's power to target opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("RakdosJoinsUp.RakdosJoinsUpValue", died_power)];

/// `getValue("creatureDied")`: the resolving trigger's creature that died, its power as it last was on the
/// battlefield (0 when there's none)
fn died_power(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some((died, _)) = game.resolving_item.as_ref().and_then(|i| i.died) else { return 0 };
    game.obj(died).power_lki.unwrap_or(0)
}
