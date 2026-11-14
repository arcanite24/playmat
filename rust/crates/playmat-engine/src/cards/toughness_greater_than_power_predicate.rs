//! `ToughnessGreaterThanPowerPredicate`: "with toughness greater than its power" (Ghalta the Immovable).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("ToughnessGreaterThanPowerPredicate", toughness_greater)];

fn toughness_greater(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let (p, t) = game.power_toughness(id);
    t > p
}
