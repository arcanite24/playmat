//! Thunder of Unity: "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)
//! I — You draw two cards and you lose 2 life.
//! II, III — Whenever a creature you control enters this turn, each opponent loses 1 life and you gain 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ThunderOfUnity.ThunderOfUnityTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(new LoseLifeOpponentsEffect(1), StaticFilters.FILTER_CONTROLLED_CREATURE)",
    creature_of_yours,
)];

/// a creature you control entering: the base trigger is all of it
fn creature_of_yours(_: &Game, _: &StackItem) -> bool {
    true
}
