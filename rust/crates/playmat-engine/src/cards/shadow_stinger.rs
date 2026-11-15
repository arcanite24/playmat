//! Shadow Stinger: "Tap another untapped Rogue you control: Shadow Stinger gains deathtouch until end of turn.
//! Whenever Shadow Stinger deals combat damage to a player, that player mills three cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ShadowStinger.ShadowStingerTriggeredAbility",
    "new DealsCombatDamageToAPlayerTriggeredAbility(new MillCardsTargetEffect(3), false, true)",
    check,
)];

/// `DAMAGED_PLAYER`, combat damage by this, its pointer at the player: the base trigger is all of it
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
