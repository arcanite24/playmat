//! Thwart the Grave: "This spell costs {1} less to cast for each creature in your party. Return target creature card
//! and up to one target Cleric, Rogue, Warrior, or Wizard creature card from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "ThwartTheGrave.ThwartTheGraveTarget",
    "new TargetCardInYourGraveyard(1, 2, StaticFilters.FILTER_CARD_CREATURE, false)",
    party_after_first,
)];

/// `possibleTargets`: once one card is chosen, only a Cleric, Rogue, Warrior or Wizard
fn party_after_first(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let others = game.chosen_before.iter().any(|t| matches!(t, Target::Obj(o) if *o != id));
    !others || ["Cleric", "Rogue", "Warrior", "Wizard"].iter().any(|s| game.has_subtype(id, s))
}
