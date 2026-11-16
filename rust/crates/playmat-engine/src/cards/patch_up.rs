//! Patch Up: "Return up to three target creature cards with total mana value 3 or less from your graveyard to the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "PatchUp.PatchUpTarget",
    "new TargetCardInYourGraveyard(0, 3, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

/// `CardUtil.checkCanTargetTotalValueLimit(…, getManaValue, 3)`
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    super::joshua_phoenixs_dominant::within_total_mana_value(game, id, 3)
}
