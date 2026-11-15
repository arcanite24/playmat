//! Rocco, Cabaretti Caterer: "When Rocco enters, if you cast it, you may search your library for a creature card with
//! mana value X or less, put it onto the battlefield, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("RoccoCabarettiCaterer.RoccoCabarettiCatererPredicate", at_most_x)];

/// Its mana value is at most the X the source was cast with (`GetXValue`: the permanent's cost tags, 0 when it
/// wasn't cast); false with no source
fn at_most_x(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let x = if game.obj(src).zone == Zone::Battlefield { game.obj(src).cast_x } else { 0 };
    (game.obj(id).def.mana_value() as i32) <= x
}
