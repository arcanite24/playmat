//! Tomebound Lich: "Deathtouch, lifelink. Whenever Tomebound Lich enters or deals combat damage to a player, draw a
//! card, then discard a card."

use super::*;

/// `ENTERS_THE_BATTLEFIELD` of this or `DAMAGED_PLAYER` by its combat damage: the two framework triggers in one
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "TomeboundLich.TomeboundLichTriggeredAbility",
    "new OrTriggeredAbility(Zone.BATTLEFIELD, new DrawDiscardControllerEffect(1, 1), new EntersBattlefieldTriggeredAbility(null), new DealsCombatDamageToAPlayerTriggeredAbility(null, false))",
    any_batch,
)];
