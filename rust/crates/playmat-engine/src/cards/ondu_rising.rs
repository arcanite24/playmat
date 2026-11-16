//! Ondu Rising: "Whenever a creature attacks this turn, it gains lifelink until end of turn. Awaken 4—{4}{W}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "OnduRising.OnduRisingTriggeredAbility",
    "new AttacksAllTriggeredAbility(new GainAbilityTargetEffect(LifelinkAbility.getInstance(), Duration.EndOfTurn), false, StaticFilters.FILTER_PERMANENT_CREATURE, SetTargetPointer.PERMANENT, false)",
    any_attacker,
)];

/// a creature attacking (`ATTACKER_DECLARED`), pointing at it: the base trigger is all of it
fn any_attacker(_: &Game, _: &StackItem) -> bool {
    true
}
