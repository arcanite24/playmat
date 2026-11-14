//! Goma Fada Vanguard: "Whenever Goma Fada Vanguard attacks, target creature an opponent controls with power less
//! than or equal to the number of Warriors you control can't block this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("GomaFadaVanguard.GomaFadaVanguardPredicate", small)];

/// power at most the Warriors the source's controller controls
fn small(game: &Game, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let owner = source.map(|s| game.obj(s).controller).unwrap_or(you);
    let warriors =
        game.battlefield.iter().filter(|p| game.obj(**p).controller == owner && game.has_subtype(**p, "Warrior")).count();
    game.power_toughness(id).0 <= warriors as i32
}
