//! Body Launderer: "Deathtouch. Whenever another nontoken creature you control dies, Body Launderer connives. When
//! Body Launderer dies, return another target non-Rogue creature card with equal or lesser power from your graveyard
//! to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "BodyLaunderer.BodyLaundererAdjuster",
    "new TargetCardInYourGraveyard(StaticFilters.FILTER_CARD_CREATURE)",
    can_target,
)];

/// `BodyLaundererAdjuster`: another non-Rogue creature card with power at most the power Body Launderer had as it
/// left the battlefield (`permanentLeftBattlefield`)
fn can_target(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let power = game.obj(src).power_lki.unwrap_or(0);
    id != src && !game.has_subtype(id, "Rogue") && game.power_toughness(id).0 <= power
}
