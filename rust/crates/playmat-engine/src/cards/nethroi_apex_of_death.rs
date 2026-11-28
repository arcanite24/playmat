//! Nethroi, Apex of Death: "Whenever this creature mutates, return any number of target creature cards with total
//! power 10 or less from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "NethroiApexOfDeath.NethroiApexOfDeathTarget",
    "new TargetCardInYourGraveyard(0, Integer.MAX_VALUE, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

/// `CardUtil.checkCanTargetTotalValueLimit` with `getPower().getValue()`: the cards chosen so far
/// (`game.chosen_before`) and `id` have total power 10 or less
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let mut total = game.power_toughness(id).0;
    for t in &game.chosen_before {
        if let Target::Obj(o) = t {
            if *o != id {
                total += game.power_toughness(*o).0;
            }
        }
    }
    total <= 10
}
