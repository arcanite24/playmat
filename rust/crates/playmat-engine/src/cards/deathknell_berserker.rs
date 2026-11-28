//! Deathknell Berserker: "When Deathknell Berserker dies, if its power was 3 or greater, create a 2/2 black Zombie
//! Berserker creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("DeathknellBerserker.DeathknellBerserkerCondtion", power_was_3)];

/// `getValue("permanentLeftBattlefield")`'s power: the power it had as it last left the battlefield
fn power_was_3(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    let power = match o.power_lki {
        Some(p) if o.zone != Zone::Battlefield => p,
        _ => game.power_toughness(source).0,
    };
    power >= 3
}
