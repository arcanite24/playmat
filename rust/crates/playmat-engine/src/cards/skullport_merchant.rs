//! Skullport Merchant: "{1}{B}, Sacrifice another creature or a Treasure: Draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("SkullportMerchant.SkullportMerchantPredicate", creature_or_treasure)];

/// a Treasure (this one too), else a creature other than the source
fn creature_or_treasure(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    if game.has_subtype(id, "Treasure") {
        return true;
    }
    source != Some(id) && game.has_type(id, CardType::Creature)
}
