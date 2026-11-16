//! Aberrant Manawurm: "Trample. Whenever you cast an instant or sorcery spell, Aberrant Manawurm gets +X/+0 until end
//! of turn, where X is the amount of mana spent to cast that spell."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("AberrantManawurm.AberrantManawurmValue", mana_spent)];

/// `ManaPaidSourceWatcher.getTotalPaid` of the spell the resolving trigger saw (`getValue("spellCast")`)
fn mana_spent(game: &Game, _: Seat, _: ObjId, _: i32) -> i32 {
    game.resolving_item.as_ref().and_then(|i| i.cast_spell).map_or(0, |s| game.obj(s).mana_spent as i32)
}
