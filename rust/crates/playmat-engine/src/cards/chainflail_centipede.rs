//! Chainflail Centipede: "Whenever Chainflail Centipede or equipped creature attacks, it gets +2/+0 until end of
//! turn. Reconfigure {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "ChainflailCentipede.ChainflailCentipedeTriggeredAbility",
    "new AttacksAllTriggeredAbility(new BoostTargetEffect(2, 0), false, StaticFilters.FILTER_PERMANENT_CREATURE, SetTargetPointer.PERMANENT, false)",
    this_or_equipped,
)];

/// the attacker pointed at is this permanent, or the creature it's attached to
fn this_or_equipped(game: &Game, t: &StackItem) -> bool {
    match t.targets.first().and_then(|g| g.first()) {
        Some(Target::Obj(a)) => *a == t.card || game.attached(t.card) == Some(*a),
        _ => false,
    }
}
