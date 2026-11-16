//! `TargetPermanentSameController(filter)`: "two target creatures controlled by the same player" (Incriminate)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::target("TargetPermanentSameController", "new TargetPermanent(2, 2, $0, false)", can_target)];

/// `canTarget`: each permanent chosen before is controlled by this one's controller
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let controller = game.obj(id).controller;
    game.chosen_before.iter().all(|t| match t {
        Target::Obj(o) => *o == id || game.obj(*o).zone != Zone::Battlefield || game.obj(*o).controller == controller,
        Target::Player(_) => true,
    })
}
