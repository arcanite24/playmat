//! Kishla Skimmer: "Flying. Whenever a card leaves your graveyard during your turn, draw a card. This ability
//! triggers only once each turn."

use super::*;

/// `ZONE_CHANGE` from your graveyard of a card you own, during your turn (`isActivePlayer`); its once-each-turn
/// limit is the card's `setTriggersLimitEachTurn(1)`, so the batch of cards leaving is the same as each card
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "KishlaSkimmer.KishlaSkimmerTriggeredAbility",
    "new CardsLeaveGraveyardTriggeredAbility(new DrawCardSourceControllerEffect(1))",
    your_turn,
)];

fn your_turn(game: &Game, item: &StackItem) -> bool {
    game.active == Some(item.controller)
}
