//! Wharf Infiltrator: "Skulk. Whenever Wharf Infiltrator deals combat damage to a player, you may draw a card. If you
//! do, discard a card. Whenever you discard a creature card, you may pay {2}. If you do, create a 3/2 colorless Eldrazi
//! Horror creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "WharfInfiltrator.WharfInfiltratorDiscardAbility",
    "new DiscardCardControllerTriggeredAbility($0, false, StaticFilters.FILTER_CARD_CREATURE)",
    always,
)];

/// `DISCARDED_CARD` by you of a creature card: the base trigger's event and filter are all of it
fn always(_game: &Game, _t: &StackItem) -> bool {
    true
}
