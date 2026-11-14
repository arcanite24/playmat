//! All-Fates Scroll: "{7}, {T}, Sacrifice this artifact: Draw X cards, where X is the number of differently named
//! lands you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("AllFatesScroll.DifferentlyNamedPermanentCount", lands)];

/// `DifferentlyNamedPermanentCount(FILTER_CONTROLLED_PERMANENT_LANDS)`: the distinct non-empty names of the lands
/// you control
fn lands(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let mut names: Vec<String> = Vec::new();
    for o in game.battlefield.iter().copied() {
        if game.obj(o).controller != you || !game.has_type(o, CardType::Land) {
            continue;
        }
        let name = game.obj(o).name().to_string();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    names.len() as i32
}
