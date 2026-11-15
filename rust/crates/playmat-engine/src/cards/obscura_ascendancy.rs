//! Obscura Ascendancy: "Whenever you cast a spell, if its mana value is equal to 1 plus the number of soul counters on
//! this enchantment, put a soul counter on it, then create a 2/2 black Spirit creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ObscuraAscendancy.ObscuraAscendancyCondition", one_more)];

/// the spell's mana value (the cast trigger's X) is 1 plus the source's soul counters (as it last was, once gone)
fn one_more(game: &Game, _: Seat, source: ObjId) -> bool {
    let Some(item) = super::liberator_urzas_battlethopter::trigger_item(game, source) else { return true };
    if item.cast_spell.is_none() {
        return false;
    }
    let o = game.obj(source);
    let souls = if o.zone == Zone::Battlefield {
        o.counter("soul")
    } else {
        o.lki_counters.iter().filter(|(n, _)| n == "soul").map(|(_, k)| *k).sum()
    };
    item.x == souls + 1
}
