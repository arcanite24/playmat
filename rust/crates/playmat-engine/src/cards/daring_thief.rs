//! Daring Thief: "Inspired — Whenever Daring Thief becomes untapped, you may exchange control of target nonland
//! permanent you control and target permanent an opponent controls that shares a card type with it."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::target(
        "DaringThief.TargetControlledPermanentSharingOpponentPermanentCardType",
        "new TargetControlledPermanent(StaticFilters.FILTER_CONTROLLED_PERMANENT_NON_LAND)",
        first,
    ),
    Handler::target("DaringThief.DaringThiefSecondTarget", "new TargetPermanent(StaticFilters.FILTER_OPPONENTS_PERMANENT)", second),
];

const TYPES: [CardType; 9] = [
    CardType::Artifact,
    CardType::Battle,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Instant,
    CardType::Kindred,
    CardType::Land,
    CardType::Planeswalker,
    CardType::Sorcery,
];

/// `Permanent.shareTypes`: a card type both have
fn share_types(game: &Game, a: ObjId, b: ObjId) -> bool {
    TYPES.iter().any(|t| game.has_type(a, *t) && game.has_type(b, *t))
}

/// a nonland permanent you control with a card type some permanent an opponent controls has
fn first(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.battlefield.iter().any(|o| game.obj(*o).controller != you && share_types(game, id, *o))
}

/// a permanent an opponent controls sharing a card type with the first target (`source.getFirstTarget()`); with no
/// first target, any
fn second(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    match game.chosen_before.first() {
        Some(Target::Obj(own)) => *own != id && share_types(game, *own, id),
        _ => true,
    }
}
