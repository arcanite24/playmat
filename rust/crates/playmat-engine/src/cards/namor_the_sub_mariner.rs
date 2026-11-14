//! Namor the Sub-Mariner: "Whenever you cast a noncreature spell with one or more blue mana symbols in its mana
//! cost, create that many 1/1 blue Merfolk creature tokens."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("NamorTheSubMariner.NamorTheSubMarinerPredicate", blue_symbols_in_cost),
    Handler::value("NamorTheSubMariner.NamorTheSubMarinerValue", blue_symbols_of_spell_cast),
];

/// the mana symbols of `id`'s mana cost that contain {U} (a hybrid one too: `containsColor`)
fn blue_symbols(game: &Game, id: ObjId) -> i32 {
    let text = &game.obj(id).def.cost.text;
    text.split('{').skip(1).filter(|s| s.split('}').next().is_some_and(|sym| sym.contains('U'))).count() as i32
}

fn blue_symbols_in_cost(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    blue_symbols(game, id) > 0
}

/// the trigger's spell (`getValue("spellCast")`): its blue mana symbols
fn blue_symbols_of_spell_cast(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.resolving_item.as_ref().and_then(|r| r.cast_spell).map_or(0, |s| blue_symbols(game, s))
}
