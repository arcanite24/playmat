//! Beamtown Beatstick: "Equipped creature gets +1/+0 and has menace. Whenever equipped creature deals combat damage
//! to a player or battle, create a Treasure token. Equip {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "BeamtownBeatstick.BeamtownBeatstickTriggeredAbility",
    "new DealsDamageToAPlayerAttachedTriggeredAbility(new CreateTokenEffect(new TreasureToken()), \"equipped creature\", false, false, true)",
    check,
)];

/// `DAMAGED_PLAYER`, combat damage, by the creature this is attached to: the base trigger's event (the engine has
/// no battles, so a battle dealt damage is left out)
fn check(_: &Game, _: &StackItem) -> bool {
    true
}
