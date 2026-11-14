//! Scytheclaw Raptor: "Whenever a player casts a spell, if it's not their turn, Scytheclaw Raptor deals 4 damage to
//! them."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("ScytheclawRaptor.ScytheclawRaptorPredicate", not_their_turn)];

/// `ScytheclawRaptorPredicate`: the spell's controller isn't the active player
fn not_their_turn(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let controller = game
        .stack
        .iter()
        .rev()
        .find(|s| s.card == id && s.ability.is_none())
        .map_or(game.obj(id).controller, |s| s.controller);
    game.active != Some(controller)
}
