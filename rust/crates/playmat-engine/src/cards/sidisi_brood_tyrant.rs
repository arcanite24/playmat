//! Sidisi, Brood Tyrant: "Whenever one or more creature cards are put into your graveyard from your library, create
//! a 2/2 black Zombie creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "SidisiBroodTyrant.SidisiBroodTyrantTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new CreateTokenEffect(new ZombieToken()), Zone.LIBRARY, Zone.GRAVEYARD, StaticFilters.FILTER_CARD_CREATURE, TargetController.YOU, true)",
    any_batch,
)];
