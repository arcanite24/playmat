//! Satoru, the Infiltrator: "Whenever Satoru and/or one or more other nontoken creatures you control enter, if none
//! of them were cast or no mana was spent to cast them, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SatoruTheInfiltrator.SatoruTheInfiltratorTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new DrawCardSourceControllerEffect(1), null, Zone.BATTLEFIELD, StaticFilters.FILTER_CREATURE_NON_TOKEN, TargetController.YOU, false)",
    no_mana_spent,
)];

/// none of the permanents that entered was a spell cast with mana spent on it
fn no_mana_spent(game: &Game, t: &StackItem) -> bool {
    t.batch.iter().all(|(o, _, from)| *from != Zone::Stack || game.obj(*o).mana_spent == 0)
}
