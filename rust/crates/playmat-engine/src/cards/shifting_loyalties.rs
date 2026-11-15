//! Shifting Loyalties: "Exchange control of two target permanents that share a card type."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "ShiftingLoyalties.TargetPermanentsThatShareCardType",
    "new TargetPermanent(2, 2, StaticFilters.FILTER_PERMANENT, false)",
    shares_a_card_type,
)];

/// `canTarget`: with one chosen, this shares a card type with the first (`MageObject.shareTypes`)
pub(crate) fn shares_a_card_type(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let Some(Target::Obj(first)) = game.chosen_before.first().copied() else { return true };
    if first == id {
        return true;
    }
    if game.obj(first).zone != Zone::Battlefield {
        return false;
    }
    [
        CardType::Artifact,
        CardType::Battle,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Instant,
        CardType::Kindred,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Sorcery,
    ]
    .into_iter()
    .any(|t| game.has_type(first, t) && game.has_type(id, t))
}
