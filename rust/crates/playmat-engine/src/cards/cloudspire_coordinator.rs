//! Cloudspire Coordinator: "When this creature enters, scry 2. {T}: Create X 1/1 colorless Pilot creature tokens,
//! where X is the number of Mounts and/or Vehicles that entered the battlefield under your control this turn. The
//! tokens have "This token saddles Mounts and crews Vehicles as though its power were 2 greater.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("CloudspireCoordinator.CloudspireCoordinatorValue", entered)];

/// `CloudspireCoordinatorWatcher`: the Mounts and Vehicles that entered the battlefield under your control this turn,
/// those that have left since too (each object keeps the turn it last entered and who controlled it; one that
/// entered twice this turn counts once)
fn entered(game: &Game, you: Seat, _source: ObjId, _x: i32) -> i32 {
    let turn = game.turn;
    game.objs
        .iter()
        .enumerate()
        .filter(|(i, o)| {
            o.entered_turn == turn
                && o.controller == you
                && if o.zone == Zone::Battlefield {
                    let id = ObjId(*i as u32);
                    game.has_subtype(id, "Mount") || game.has_subtype(id, "Vehicle")
                } else {
                    o.def.has_subtype("Mount") || o.def.has_subtype("Vehicle")
                }
        })
        .count() as i32
}
