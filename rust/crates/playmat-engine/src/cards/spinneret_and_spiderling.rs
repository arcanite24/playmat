//! Spinneret and Spiderling: "Whenever Spinneret and Spiderling deals 4 or more damage, exile the top card of your
//! library. Until the end of your next turn, you may play that card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SpinneretAndSpiderling.SpinneretAndSpiderlingTriggeredAbility",
    "new DealsDamageSourceTriggeredAbility(new ExileTopXMayPlayUntilEffect(1, Duration.UntilEndOfYourNextTurn))",
    four_or_more,
)];

/// `DealsDamageSourceTriggeredAbility.checkTrigger` and the batch's amount at least 4 (the trigger's X)
fn four_or_more(_: &Game, item: &StackItem) -> bool {
    item.x >= 4
}
