//! Thunder Salvo: "Thunder Salvo deals X damage to target creature, where X is 2 plus the number of other spells
//! you've cast this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("ThunderSalvo.ThunderSalvoValue", other_spells)];

/// `SpellsCastWatcher`: the spells you've cast this turn but this one
fn other_spells(game: &Game, you: Seat, source: ObjId, _x: i32) -> i32 {
    game.watch.spells.iter().filter(|s| s.caster == you && s.card != source).count() as i32
}
