//! Dancing from Dark to Dawn: "Whenever you cast a creature spell, put X +1/+1 counters on target creature you
//! control, where X is that spell's mana value."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("DancingFromDarkToDawn.DancingFromDarkToDawnValue", spell_mana_value)];

/// the cast spell's mana value (`getValue("spellCast")`): the cast trigger's X
fn spell_mana_value(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    match game.resolving_item.as_ref() {
        Some(item) if item.cast_spell.is_some() => item.x,
        _ => 0,
    }
}
