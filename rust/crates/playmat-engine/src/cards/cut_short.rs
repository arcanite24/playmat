//! Cut Short: "Convoke. Destroy target planeswalker that was activated this turn or tapped creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("CutShort.CutShortPredicate", activated_or_tapped)];

/// a tapped creature, or a planeswalker one of whose abilities was activated this turn (`CutShortWatcher`'s
/// `ACTIVATED_ABILITY` sources: its loyalty abilities, as the object it is now)
fn activated_or_tapped(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let o = game.obj(id);
    game.has_type(id, CardType::Creature) && o.tapped || game.has_type(id, CardType::Planeswalker) && o.loyalty_used > 0
}
