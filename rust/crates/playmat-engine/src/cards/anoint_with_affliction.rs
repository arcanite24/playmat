//! Anoint with Affliction: "Exile target creature if it has mana value 3 or less. Corrupted — Exile that creature
//! instead if its controller has three or more poison counters."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AnointWithAffliction.AnointWithAfflictionEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let owner = game.obj(o).controller;
    let poison: i32 = game.player(owner).counters.iter().filter(|(k, _)| k == "poison").map(|(_, n)| *n).sum();
    if game.obj(o).def.mana_value() <= 3 || poison >= 3 {
        game.move_to(o, Zone::Exile);
    }
    Some(())
}
