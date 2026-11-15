//! Magmablood Archaic: "Whenever you cast an instant or sorcery spell, creatures you control get +1/+0 until end of
//! turn for each color of mana spent to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("MagmabloodArchaic.MagmabloodArchaicManaSpentValue", spell_colors_spent)];

/// the colors of mana spent to cast the spell the trigger saw (`getValue("spellCast")`)
fn spell_colors_spent(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(spell) = game.resolving_item.as_ref().and_then(|i| i.cast_spell) else { return 0 };
    game.obj(spell).spent_colors.iter().filter(|n| **n > 0).count() as i32
}
