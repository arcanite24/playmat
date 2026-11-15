//! `ControllerIsActivePlayerPredicate` and `TargetController.ACTIVE.getControllerPredicate()`: a permanent the active
//! player controls (Tooth Collector, Topplegeist, Sentinel of the Eternal Watch; `INACTIVE` is its negation)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("ControllerIsActivePlayerPredicate", active_controls)];

/// `game.isActivePlayer(input.getControllerId())`
fn active_controls(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    game.active == Some(game.obj(id).controller)
}
