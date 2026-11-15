//! Symmetry Matrix: "Whenever a creature you control with power equal to its toughness enters, you may pay {1}. If
//! you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("SymmetryMatrix.SymmetryMatrixPredicate", square)];

/// its power equals its toughness
fn square(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let (p, t) = game.power_toughness(id);
    p == t
}
