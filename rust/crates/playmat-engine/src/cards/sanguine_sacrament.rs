//! Sanguine Sacrament: "You gain twice X life. Put Sanguine Sacrament on the bottom of its owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SanguineSacrament.SanguineSacramentValue", twice_x)];

/// `getSourceCostsTag(game, sourceAbility, "X", 0) * 2`
fn twice_x(_: &Game, _: Seat, _: ObjId, x: i32) -> i32 {
    x * 2
}
