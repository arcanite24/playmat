//! Rielle, the Everwise: "Whenever you discard one or more cards for the first time each turn, draw that many
//! cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RielleTheEverwise.RielleTheEverwiseTriggeredAbility",
    "new DiscardOneOrMoreCardsTriggeredAbility(new DrawCardSourceControllerEffect(SavedDiscardValue.MANY))",
    first_time,
)];

/// `RielleTheEverwiseWatcher.checkDiscarded`: this is your first `DISCARDED_CARDS` this turn (the watcher has seen
/// it already as it triggers); checked only as it triggers
fn first_time(game: &Game, item: &StackItem) -> bool {
    if game.resolving_item.as_ref().is_some_and(|r| r.card == item.card && r.ability == item.ability) {
        return true;
    }
    game.watch.discard_batches.iter().filter(|s| **s == item.controller).count() < 2
}
