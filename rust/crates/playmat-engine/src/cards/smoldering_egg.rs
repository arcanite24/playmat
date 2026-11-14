//! Smoldering Egg: "Whenever you cast an instant or sorcery spell, put a number of ember counters on Smoldering Egg
//! equal to the amount of mana spent to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("SmolderingEgg.SmolderingEggValue", mana_spent_on_that_spell)];

/// the mana spent to cast the spell the resolving trigger saw (`effect.getValue("spellCast")`), 0 without one
fn mana_spent_on_that_spell(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.resolving_item.as_ref().and_then(|i| i.cast_spell).map_or(0, |s| game.obj(s).mana_spent as i32)
}
