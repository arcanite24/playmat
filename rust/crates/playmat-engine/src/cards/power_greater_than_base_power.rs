//! `PowerGreaterThanBasePowerPredicate` (Kutzil, Malamet Exemplar: "creatures you control each with power greater
//! than its base power")

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("PowerGreaterThanBasePowerPredicate", apply)];

fn apply(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.power_toughness(id).0 > game.base_power_toughness(id).0
}
