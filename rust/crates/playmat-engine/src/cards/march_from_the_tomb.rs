//! March from the Tomb: "Return any number of target Ally creature cards with total mana value 8 or less from your
//! graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "MarchFromTheTomb.MarchFromTheTombTarget",
    "new TargetCardInYourGraveyard(0, Integer.MAX_VALUE, filterStatic)",
    can_target,
)];

/// with the targets chosen so far, a total mana value of 8 or less
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    super::joshua_phoenixs_dominant::within_total_mana_value(game, id, 8)
}
