//! Rumor Gatherer: "Whenever another creature you control enters, scry 1. If this is the second time this ability
//! has resolved this turn, draw a card instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("RumorGatherer.RumorGathererCondition", second_time)];

/// `AbilityResolvedWatcher.getResolutionCount == 2`, by the source alone: its Alliance trigger is its only ability
/// that resolves
fn second_time(game: &Game, _you: Seat, source: ObjId) -> bool {
    let (turn, now) = (game.turn, game.obj(source).zcc);
    game.abilities_resolved.iter().filter(|r| r.0 == source && r.2 == now && r.3 == turn).count() == 2
}
