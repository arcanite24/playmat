//! Elite Scaleguard: "When Elite Scaleguard enters, bolster 2. Whenever a creature you control with a +1/+1 counter
//! on it attacks, tap target creature defending player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "EliteScaleguard.EliteScaleguardTargetAdjuster",
    "new TargetPermanent(StaticFilters.FILTER_PERMANENT_CREATURE)",
    can_target,
)];

/// `EliteScaleguardTargetAdjuster`: a creature the defending player controls (in a duel, your opponent)
fn can_target(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.obj(id).controller != you
}
