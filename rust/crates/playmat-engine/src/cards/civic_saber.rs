//! Civic Saber: "Equipped creature gets +1/+0 for each of its colors. Equip {1}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("CivicSaber.CivicSaberColorCount", colors_of_equipped)];

/// The colors of the creature the Equipment is attached to, while both are on the battlefield
fn colors_of_equipped(game: &Game, _you: Seat, source: ObjId, _x: i32) -> i32 {
    if game.obj(source).zone != Zone::Battlefield {
        return 0;
    }
    let Some((to, zcc)) = game.obj(source).attached_to else { return 0 };
    if game.obj(to).zone != Zone::Battlefield || game.obj(to).zcc != zcc {
        return 0;
    }
    game.colors(to).len() as i32
}
