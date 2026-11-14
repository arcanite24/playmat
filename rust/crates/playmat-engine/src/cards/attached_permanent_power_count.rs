//! `AttachedPermanentPowerCount`: the power of the permanent the source is attached to, or was attached to as it
//! left (Dying Wish: "target player loses X life and you gain X life, where X is its power").

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("AttachedPermanentPowerCount", attached_power)];

/// `getPermanentOrLKIBattlefield(source.getAttachedTo())`'s power, at least 0 as Java's count is
fn attached_power(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    let src = game.obj(source);
    let to = if src.zone == Zone::Battlefield { src.attached_to } else { src.lki_attached_to };
    let Some((o, zcc)) = to else { return 0 };
    let p = game.obj(o);
    let power = if p.zone == Zone::Battlefield && p.zcc == zcc {
        game.power_toughness(o).0
    } else {
        p.power_lki.unwrap_or(0)
    };
    power.max(0)
}
