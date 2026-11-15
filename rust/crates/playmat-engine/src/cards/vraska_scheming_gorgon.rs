//! Vraska, Scheming Gorgon: "+2: Creatures you control get +1/+0 until end of turn. −3: Destroy target creature.
//! −10: Until end of turn, creatures you control gain deathtouch and "Whenever this creature deals damage to an
//! opponent, that player loses the game.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "VraskaSchemingGorgon.VraskaSchemingGorgonTriggeredAbility",
    "new DealsCombatDamageToAPlayerTriggeredAbility($0, false, true)",
    check,
)];

/// `DAMAGED_PLAYER` by this creature, combat damage (the base trigger, its pointer at the player), the player an
/// opponent
fn check(_: &Game, t: &StackItem) -> bool {
    t.targets.iter().flatten().any(|x| matches!(x, Target::Player(p) if *p != t.controller))
}
