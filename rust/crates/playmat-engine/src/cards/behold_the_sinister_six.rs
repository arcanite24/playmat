//! Behold the Sinister Six!: "Return up to six target creature cards with different names from your graveyard to the
//! battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "BeholdTheSinisterSix.BeholdTheSinisterSixTarget",
    "new TargetCardInYourGraveyard(0, 6, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

/// `canTarget` and `possibleTargets`: no card chosen so far has its name
fn can_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let name = game.obj(id).name();
    !game.chosen_before.iter().any(|t| matches!(t, Target::Obj(o) if *o != id && game.obj(*o).name() == name))
}
