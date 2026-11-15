//! Raise the Draugr: "Choose one — • Return target creature card from your graveyard to your hand. • Return two
//! target creature cards that share a creature type from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "RaiseTheDraugr.RaiseTheDraugrTarget",
    "new TargetCardInYourGraveyard(2, 2, StaticFilters.FILTER_CARD_CREATURE, false)",
    can_target,
)];

/// `canTarget`: the second shares a creature type with the first chosen. With none chosen yet, a card with another
/// creature card in your graveyard to share one with, so the mode can be chosen only when two cards do (Java's
/// `canChoose`), and each chosen card still has its partner as the spell resolves
fn can_target(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let share = |a: ObjId, b: ObjId| super::invasion_of_new_capenna::share_creature_types(game, a, b);
    match game.chosen_before.first() {
        Some(Target::Obj(first)) if *first != id => share(*first, id),
        _ => game
            .player(you)
            .graveyard
            .iter()
            .any(|c| *c != id && game.has_type(*c, CardType::Creature) && share(*c, id)),
    }
}
