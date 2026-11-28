//! Loki, God of Mischief: "Whenever a player or permanent becomes the target of an ability you control, draw a card.
//! This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "LokiGodOfMischief.LokiGodOfMischiefTriggeredAbility",
    "new BecomesTargetPlayerOrPermanentTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_PERMANENT, StaticFilters.FILTER_SPELL_OR_ABILITY_A, TargetController.YOU, SetTargetPointer.NONE, TargetController.ANY, true, true).setTriggersLimitEachTurn(1)",
    any_batch,
)];
