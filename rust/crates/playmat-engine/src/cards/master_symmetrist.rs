//! Master Symmetrist: "Whenever a creature you control with power equal to its toughness attacks, it gains trample
//! until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("MasterSymmetrist.MasterSymmetristPredicate", symmetric)];

fn symmetric(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let (p, t) = game.power_toughness(id);
    p == t
}
