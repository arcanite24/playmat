//! Towering Titan: "Towering Titan enters with X +1/+1 counters on it, where X is the total toughness of other
//! creatures you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("ToweringTitan.ToweringTitanCount", toughness)];

/// The total toughness of the other creatures you control
fn toughness(game: &Game, you: Seat, source: ObjId, _: i32) -> i32 {
    game.battlefield
        .iter()
        .copied()
        .filter(|o| *o != source && game.obj(*o).controller == you && game.has_type(*o, CardType::Creature))
        .map(|o| game.power_toughness(o).1)
        .sum()
}
