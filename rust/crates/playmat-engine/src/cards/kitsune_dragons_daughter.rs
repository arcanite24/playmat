//! Kitsune, Dragon's Daughter: "Vigilance. Whenever Kitsune enters or deals combat damage to a player, you may
//! exchange control of two other target creatures controlled by different players."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "KitsuneDragonsDaughter.KitsuneDragonsDaughterTarget",
    "new TargetPermanent(2, 2, filter, false)",
    different_controller,
)];

/// `canTarget`: no permanent chosen before (other than this one) has this one's controller
fn different_controller(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let controller = game.obj(id).controller;
    !game.chosen_before.iter().any(|t| match t {
        Target::Obj(o) => *o != id && game.obj(*o).zone == Zone::Battlefield && game.obj(*o).controller == controller,
        Target::Player(_) => false,
    })
}
