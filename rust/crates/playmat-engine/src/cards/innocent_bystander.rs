//! Innocent Bystander: "Whenever Innocent Bystander is dealt 3 or more damage, investigate."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "InnocentBystander.InnocentBystanderTriggeredAbility",
    "new DealtDamageToSourceTriggeredAbility(new InvestigateEffect(), false)",
    three_or_more,
)];

/// the damage of the batch dealt to it (`DAMAGED_BATCH_FOR_ONE_PERMANENT`'s amount) is 3 or more
fn three_or_more(_: &Game, item: &StackItem) -> bool {
    item.x >= 3
}
