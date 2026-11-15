//! `MaxManaValueControlledCreatureOrPlaneswalkerPredicate` (Soul Shatter): a creature or planeswalker with the
//! greatest mana value among the creatures and planeswalkers its controller controls.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("MaxManaValueControlledCreatureOrPlaneswalkerPredicate", max)];

fn max(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let owner = game.obj(id).controller;
    let best = game
        .battlefield
        .iter()
        .filter(|p| {
            game.obj(**p).controller == owner
                && (game.has_type(**p, CardType::Creature) || game.has_type(**p, CardType::Planeswalker))
        })
        .map(|p| game.obj(*p).def.mana_value())
        .max()
        .unwrap_or(0);
    game.obj(id).def.mana_value() >= best
}
