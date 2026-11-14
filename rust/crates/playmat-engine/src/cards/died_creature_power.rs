//! "equal to that creature's power" for a "whenever a creature dies" trigger (Java's `getValue("creatureDied")`, its
//! power as it last was on the battlefield): Vincent Valentine, Death's Presence.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("VincentValentine.VincentValentineValue", died_power),
    Handler::value("DeathsPresence.DeathsPresenceDiedPermanentPowerCount", died_power),
];

/// the resolving trigger's creature that died: its power as it last was on the battlefield (0 when there's none)
fn died_power(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some((died, _)) = game.resolving_item.as_ref().and_then(|i| i.died) else { return 0 };
    game.obj(died).power_lki.unwrap_or(0)
}
