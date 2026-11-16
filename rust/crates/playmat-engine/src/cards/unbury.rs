//! Unbury: "Choose one — Return target creature card from your graveyard to your hand. / Return two target creature
//! cards that share a creature type from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "Unbury.UnburyTarget",
    "new TargetCardInYourGraveyard(2, 2, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

/// `canTarget`: with a card chosen, one that shares a creature type with it (`shareCreatureTypes`). Before that (and
/// checked on its own), a card with another creature card in your graveyard to share a type with, which is also
/// `canChoose`: the mode's targets can be chosen only with such a pair. (Java lets a first card with no partner be
/// picked; here it can't.)
fn can_target(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let share = super::invasion_of_new_capenna::share_creature_types;
    match game.chosen_before.iter().find_map(|t| if let Target::Obj(o) = t { Some(*o) } else { None }) {
        Some(first) if first != id => share(game, first, id),
        _ => game
            .player(you)
            .graveyard
            .iter()
            .any(|c| *c != id && game.obj(*c).is(CardType::Creature) && share(game, id, *c)),
    }
}
