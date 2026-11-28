//! Stonebinder's Familiar: "Whenever one or more cards are put into exile during your turn, put a +1/+1 counter on
//! Stonebinder's Familiar. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "StonebindersFamiliar.StonebindersFamiliarTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new AddCountersSourceEffect(CounterType.P1P1.createInstance()), null, Zone.EXILED, StaticFilters.FILTER_CARD, TargetController.ANY, true).setTriggersLimitEachTurn(1)",
    your_turn,
)];

/// A card (a token from the battlefield doesn't count: `cards`) put into exile (the base trigger, once each turn),
/// while you're the active player
fn your_turn(game: &Game, t: &StackItem) -> bool {
    game.active == Some(t.controller)
}
