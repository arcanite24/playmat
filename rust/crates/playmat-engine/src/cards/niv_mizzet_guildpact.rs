//! Niv-Mizzet, Guildpact: "Whenever Niv-Mizzet deals combat damage to a player, it deals X damage to any target,
//! target player draws X cards, and you gain X life, where X is the number of different color pairs among permanents
//! you control that are exactly two colors."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("NivMizzetGuildpact.NivMizzetGuildpactCount", pairs)];

/// the distinct two-color combinations among the permanents you control
fn pairs(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut seen: Vec<Vec<crate::card::Color>> = Vec::new();
    for o in game.battlefield.iter().copied() {
        if game.obj(o).controller != you {
            continue;
        }
        let mut colors = game.colors(o);
        colors.sort();
        colors.dedup();
        if colors.len() == 2 && !seen.contains(&colors) {
            seen.push(colors);
        }
    }
    seen.len() as i32
}
