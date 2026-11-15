//! Hedge Shredder: "Whenever one or more land cards are put into your graveyard from your library, put them onto
//! the battlefield tapped."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "HedgeShredder.HedgeShredderTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new ReturnFromGraveyardToBattlefieldTargetEffect(true), Zone.LIBRARY, Zone.GRAVEYARD, StaticFilters.FILTER_CARD_LAND, TargetController.YOU, true)",
    any_batch,
)];
