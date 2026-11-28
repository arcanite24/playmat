//! Titania, Voice of Gaea: "Reach. Whenever one or more land cards are put into your graveyard from anywhere, you
//! gain 2 life. At the beginning of your upkeep, if there are four or more land cards in your graveyard and you both
//! own and control Titania, Voice of Gaea and a land named Argoth, Sanctum of Nature, exile them, then meld them into
//! Titania, Gaea Incarnate."

use super::*;

/// (`checkEvent`: a land card, by `game.getCard`, its owner the controller)
pub const HANDLERS: &[Handler] = &[Handler::trigger(
    "TitaniaVoiceOfGaea.TitaniaVoiceOfGaeaTriggeredAbility",
    "new ZoneChangeBatchTriggeredAbility(new GainLifeEffect(2), null, Zone.GRAVEYARD, StaticFilters.FILTER_CARD_LAND, TargetController.YOU, true)",
    any_batch,
)];
