//! Ghirapur Orrery: "Each player may play an additional land on each of their turns. At the beginning of each
//! player's upkeep, if that player has no cards in hand, that player draws three cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("GhirapurOrrery.CardsInHandCondition", active_has_none)];

/// `CardsInHandCondition(ComparisonType.EQUAL_TO, 0, TargetController.ACTIVE)`: the active player's hand is empty
fn active_has_none(game: &Game, _you: Seat, _source: ObjId) -> bool {
    game.active.is_some_and(|a| game.player(a).hand.is_empty())
}
