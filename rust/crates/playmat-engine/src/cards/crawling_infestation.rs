//! Crawling Infestation: "Whenever one or more creature cards are put into your graveyard from anywhere during your
//! turn, create a 1/1 green Insect creature token. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CrawlingInfestation.CrawlingInfestationTriggeredAbility",
    "new PutCardIntoGraveFromAnywhereAllTriggeredAbility(new CreateTokenEffect(new InsectToken()), false, StaticFilters.FILTER_CARD_CREATURE, TargetController.YOU).setTriggersLimitEachTurn(1)",
    your_turn,
)];

/// `checkTrigger`: during your turn
fn your_turn(game: &Game, item: &StackItem) -> bool {
    game.active == Some(item.controller)
}
