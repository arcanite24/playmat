//! Pompous Gadabout: "During your turn, Pompous Gadabout has hexproof. Pompous Gadabout can't be blocked by creatures
//! that don't have a name."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("PompousGadabout.PompousGadaboutPredicate", no_name)];

/// a permanent without a name (a face-down creature, a nameless token)
fn no_name(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).name().is_empty()
}
