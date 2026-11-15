//! Campaign of Vengeance: "Whenever a creature you control attacks, defending player loses 1 life and you gain 1
//! life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "CampaignOfVengeance.CampaignOfVengeanceTriggeredAbility",
    "new AttacksAllTriggeredAbility(new LoseLifeTargetEffect(1), false, StaticFilters.FILTER_CONTROLLED_CREATURE, SetTargetPointer.PLAYER, false)",
    any_batch,
)];
