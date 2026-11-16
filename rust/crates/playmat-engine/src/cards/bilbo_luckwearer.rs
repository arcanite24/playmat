//! Bilbo, Luckwearer // Burglar's Plot: "Bilbo can't be blocked. Whenever Bilbo deals combat damage to a player, draw
//! a card, then discard a card. — Burglar's Plot: Exchange control of two target nonland permanents that share a card
//! type."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "BilboLuckwearer.TargetPermanentsThatShareCardType",
    "new TargetPermanent(2, 2, StaticFilters.FILTER_PERMANENT_NON_LAND, false)",
    super::shifting_loyalties::shares_a_card_type,
)];
