//! Sixth Sense: "Enchant creature. Enchanted creature has "Whenever this creature deals combat damage to a player,
//! you may draw a card.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SixthSense.SixthSenseTriggeredAbility",
    "new DealsCombatDamageToAPlayerTriggeredAbility(new DrawCardSourceControllerEffect(1), true)",
    always,
)];

/// `DAMAGED_PLAYER`, combat damage from this creature: the base trigger's event is all of it
fn always(_game: &Game, _t: &StackItem) -> bool {
    true
}
