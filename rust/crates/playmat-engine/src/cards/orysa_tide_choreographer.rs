//! Orysa, Tide Choreographer: "This spell costs {3} less to cast if creatures you control have total toughness 10 or
//! greater."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("OrysaTideChoreographer.OrysaTideChoreographerCondition", tough)];

/// `OrysaTideChoreographerValue`: the total toughness of the creatures you control
fn tough(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_type(**o, CardType::Creature))
        .map(|o| game.power_toughness(*o).1)
        .sum::<i32>()
        >= 10
}
