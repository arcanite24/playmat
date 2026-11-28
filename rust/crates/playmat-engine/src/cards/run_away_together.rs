//! Run Away Together: "Choose two target creatures controlled by different players. Return those creatures to their
//! owners' hands."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "RunAwayTogether.RunAwayTogetherTarget",
    "new TargetPermanent(2, 2, StaticFilters.FILTER_PERMANENT_CREATURE, false)",
    can_target,
)];

/// `canTarget`: no other creature chosen so far is controlled by this one's controller
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let controller = game.obj(id).controller;
    !game.chosen_before.iter().any(|t| match t {
        Target::Obj(o) => *o != id && game.obj(*o).zone == Zone::Battlefield && game.obj(*o).controller == controller,
        Target::Player(_) => false,
    })
}
