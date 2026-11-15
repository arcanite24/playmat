//! Fiendish Panda: "Whenever you gain life, put a +1/+1 counter on this creature. When this creature dies, return
//! another target non-Bear creature card with mana value less than or equal to this creature's power from your
//! graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "FiendishPanda.FiendishPandaAdjuster",
    "new TargetCardInYourGraveyard(StaticFilters.FILTER_CARD_CREATURE)",
    can_target,
)];

/// `FiendishPandaAdjuster`: another non-Bear creature card with mana value at most the power the Panda had as it
/// left the battlefield (`permanentLeftBattlefield`)
fn can_target(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let power = game.obj(src).power_lki.unwrap_or(0);
    id != src && !game.has_subtype(id, "Bear") && game.obj(id).def.mana_value() as i32 <= power
}
