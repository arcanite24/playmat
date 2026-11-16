//! Raiders' Spoils: "Creatures you control get +1/+0. Whenever a Warrior you control deals combat damage to a player,
//! you may pay 1 life. If you do, draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "RaidersSpoils.RaidersSpoilsTriggeredAbility",
    "new DealsDamageToAPlayerAllTriggeredAbility(new DoIfCostPaid(new DrawCardSourceControllerEffect(1), new PayLifeCost(1)), new FilterControlledPermanent(SubType.WARRIOR, \"a Warrior you control\"), false, SetTargetPointer.NONE, true)",
    any_batch,
)];
