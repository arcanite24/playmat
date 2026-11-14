//! Ajani Unrelenting: "−3: Ajani deals 4 damage to each creature except for tokens you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("AjaniUnrelenting.AjaniUnrelentingPredicate", not_your_token)];

/// Not a token, or not controlled by the filter's player
fn not_your_token(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    !o.token || o.controller != you
}
