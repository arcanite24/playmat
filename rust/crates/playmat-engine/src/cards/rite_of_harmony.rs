//! Rite of Harmony: "Whenever a creature or enchantment you control enters this turn, draw a card. Flashback
//! {2}{G}{W}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RiteOfHarmony.RiteOfHarmonyTriggeredAbility",
    "new EntersBattlefieldControlledTriggeredAbility(new DrawCardSourceControllerEffect(1), filter)",
    of_filter,
)];

/// a creature or enchantment you control entering: the base trigger is all of it
fn of_filter(_: &Game, _: &StackItem) -> bool {
    true
}
