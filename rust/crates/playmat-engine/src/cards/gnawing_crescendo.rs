//! Gnawing Crescendo: "Creatures you control get +2/+0 until end of turn. Whenever a nontoken creature you control
//! dies this turn, create a 1/1 black Rat creature token with "This creature can't block.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "GnawingCrescendo.GnawingCrescendoTriggeredAbility",
    "new DiesCreatureTriggeredAbility(new CreateTokenEffect(new RatCantBlockToken()), false, StaticFilters.FILTER_CONTROLLED_CREATURE_NON_TOKEN)",
    nontoken_of_yours,
)];

/// a nontoken creature you control dying: the base trigger is all of it
fn nontoken_of_yours(_: &Game, _: &StackItem) -> bool {
    true
}
