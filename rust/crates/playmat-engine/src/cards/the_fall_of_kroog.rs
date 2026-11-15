//! The Fall of Kroog: "Choose target opponent. Destroy target land that player controls. The Fall of Kroog deals 3
//! damage to that player and 1 damage to each creature they control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "TheFallOfKroog.TheFallOfKroogTarget",
    "new TargetPermanent(1, 1, StaticFilters.FILTER_LAND, false)",
    controlled_by_first_target,
)];

/// `canTarget`: a land the ability's first target (the opponent) controls; with no first target to compare with
/// (checked again as the spell resolves), it holds
fn controlled_by_first_target(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Player(p)) => game.obj(id).controller == *p,
        Some(Target::Obj(_)) => false,
        None => true,
    }
}
