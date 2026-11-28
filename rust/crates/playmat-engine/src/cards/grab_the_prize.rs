//! Grab the Prize: "As an additional cost to cast this spell, discard a card. Draw two cards. If the discarded card
//! wasn't a land card, Grab the Prize deals 2 damage to each opponent."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("GrabThePrize.GrabThePrizeCondition", discarded_nonland)];

/// a card its `DiscardCardCost` discarded isn't a land card (the resolving spell's, or the one on the stack)
fn discarded_nonland(game: &Game, _you: Seat, source: ObjId) -> bool {
    game.resolving_item
        .as_ref()
        .filter(|i| i.card == source && i.ability.is_none())
        .or_else(|| game.stack.iter().rev().find(|i| i.card == source && i.ability.is_none()))
        .is_some_and(|i| i.cost_discarded.iter().any(|c| !game.obj(*c).def.is(CardType::Land)))
}
