//! `TransformedPredicate`: a transformed permanent (`Permanent.isTransformed`: Mutagen Connoisseur, Oculus Whelp)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("TransformedPredicate", transformed)];

/// a permanent showing its back face: a transforming double-faced card's (whether it transformed there or entered
/// transformed), or a modal double-faced card's that transformed in place; not one cast as its back face
fn transformed(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    o.zone == Zone::Battlefield
        && o.front.as_ref().is_some_and(|f| o.transformed_in_place || f.transform_back.is_some())
}
