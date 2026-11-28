//! Reckoner's Bargain: "As an additional cost to cast this spell, sacrifice an artifact or creature. You gain life
//! equal to the sacrificed permanent's mana value and draw two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("ReckonersBargain.ReckonersBargainValue", value)];

/// the mana value of what the cost sacrificed (`SacrificeCostManaValue`)
fn value(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.resolving_item.as_ref().map_or(0, |i| i.sacrificed_stats[2])
}
