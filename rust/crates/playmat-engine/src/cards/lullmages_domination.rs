//! Lullmage's Domination: "This spell costs {3} less to cast if it targets a creature whose controller has eight or
//! more cards in their graveyard. Gain control of target creature with mana value X."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("LullmagesDomination.LullmagesDominationPredicate", eight_in_graveyard)];

/// Its controller has eight or more cards in their graveyard
fn eight_in_graveyard(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.player(game.obj(id).controller).graveyard.len() >= 8
}
