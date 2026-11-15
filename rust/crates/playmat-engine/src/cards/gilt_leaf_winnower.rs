//! Gilt-Leaf Winnower: "When this creature enters, you may destroy target non-Elf creature whose power and
//! toughness aren't equal."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("GiltLeafWinnower.PowerToughnessNotEqualPredicate", power_toughness_not_equal)];

fn power_toughness_not_equal(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let (p, t) = game.power_toughness(id);
    p != t
}
