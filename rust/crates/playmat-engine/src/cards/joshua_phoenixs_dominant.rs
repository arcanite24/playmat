//! Joshua, Phoenix's Dominant // Phoenix, Warden of Fire: chapter III, "Flames of Rebirth": "Return any number of
//! target creature cards with total mana value 6 or less from your graveyard to the battlefield. Exile this Saga,
//! then return it to the battlefield (front face up)."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "JoshuaPhoenixsDominant.PhoenixWardenOfFireTarget",
    "new TargetCardInYourGraveyard(0, Integer.MAX_VALUE, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    within_total_mana_value(game, id, 6)
}

/// `CardUtil.checkCanTargetTotalValueLimit` (and `checkPossibleTargetsTotalValueLimit`) with
/// `MageObject::getManaValue`: the cards chosen so far (`game.chosen_before`) and `id` come to `max` or less in all
pub(super) fn within_total_mana_value(game: &Game, id: ObjId, max: u32) -> bool {
    let mut total = game.obj(id).def.mana_value();
    for t in &game.chosen_before {
        if let Target::Obj(o) = t {
            if *o != id {
                total += game.obj(*o).def.mana_value();
            }
        }
    }
    total <= max
}
