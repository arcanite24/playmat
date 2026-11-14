//! The Gitrog Monster: "Whenever one or more land cards are put into your graveyard from anywhere, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "TheGitrogMonster.TheGitrogMonsterTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new DrawCardSourceControllerEffect(1), null, Zone.GRAVEYARD, StaticFilters.FILTER_CARD_LAND, TargetController.YOU, true)",
    any_batch,
)];
