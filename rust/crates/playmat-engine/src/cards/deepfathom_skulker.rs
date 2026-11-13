//! Deepfathom Skulker: "Whenever a creature you control deals combat damage to a player, you may draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "DeepfathomSkulker.DeepfathomSkulkerTriggeredAbility",
    "new DealsDamageToAPlayerAllTriggeredAbility(new DrawCardSourceControllerEffect(1), StaticFilters.FILTER_CONTROLLED_CREATURE, true, SetTargetPointer.NONE, true)",
    check,
)];

/// The trigger is the framework one's: a creature you control dealing combat damage to a player
fn check(_game: &Game, _t: &StackItem) -> bool {
    true
}
