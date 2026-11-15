//! Volo, Guide to Monsters: "Whenever you cast a creature spell that doesn't share a creature type with a creature you
//! control or a creature card in your graveyard, copy that spell."

use super::invasion_of_new_capenna::share_creature_types;
use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("VoloGuideToMonsters.VoloGuideToMonstersPredicate", new_type)];

/// the spell shares no creature type with a creature card in its controller's graveyard nor with a creature they
/// control
fn new_type(game: &Game, _: Seat, _: Option<ObjId>, spell: ObjId) -> bool {
    let p = game.obj(spell).controller;
    let in_graveyard = game
        .player(p)
        .graveyard
        .iter()
        .any(|c| game.obj(*c).is(CardType::Creature) && share_creature_types(game, spell, *c));
    if in_graveyard {
        return false;
    }
    !game.battlefield.iter().any(|o| {
        game.obj(*o).controller == p && game.has_type(*o, CardType::Creature) && share_creature_types(game, spell, *o)
    })
}
