//! Ancient Cornucopia: "Whenever you cast a spell that's one or more colors, you may gain 1 life for each of that
//! spell's colors. Do this only once each turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("AncientCornucopia.AncientCornucopiaValue", spell_colors)];

/// the cast spell's colors (`getValue("spellCast")`)
fn spell_colors(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    let Some(spell) = game.resolving_item.as_ref().and_then(|i| i.cast_spell) else { return 0 };
    let mut colors = game.colors(spell);
    colors.sort();
    colors.dedup();
    colors.len() as i32
}
