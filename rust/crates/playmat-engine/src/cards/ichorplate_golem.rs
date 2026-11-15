//! Ichorplate Golem: "Whenever a creature you control enters, if it has one or more oil counters on it, put an oil
//! counter on it. Creatures you control with oil counters on them get +1/+1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "IchorplateGolem.IchorplateGolemTriggeredAbility",
    "new EntersBattlefieldAllTriggeredAbility(Zone.BATTLEFIELD, new AddCountersTargetEffect(CounterType.OIL.createInstance()), StaticFilters.FILTER_CONTROLLED_PERMANENT, false, SetTargetPointer.PERMANENT)",
    oiled,
)];

/// `checkTrigger`: the permanent you control that entered (any permanent, as Java reads it) has an oil counter
fn oiled(game: &Game, item: &StackItem) -> bool {
    item.entering.is_some_and(|o| game.obj(o).counters.iter().any(|(n, c)| n == "oil" && *c > 0))
}
