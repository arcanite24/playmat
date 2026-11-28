//! Crawling Sensation: "Whenever one or more land cards are put into your graveyard from anywhere for the first time
//! each turn, create a 1/1 green Insect creature token."

use super::*;

/// (Java keeps the turn it last triggered in the game state: once each turn)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CrawlingSensation.CrawlingSensationTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new CreateTokenEffect(new InsectToken()), null, Zone.GRAVEYARD, StaticFilters.FILTER_CARD_LAND, TargetController.YOU, true).setTriggersLimitEachTurn(1)",
    any_batch,
)];
