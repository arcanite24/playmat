//! Vraska, Betrayal's Sting: "−9: If target player has fewer than nine poison counters, they get a number of poison
//! counters equal to the difference."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VraskaBetrayalsSting.VraskaBetrayalsStingEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let poison: i32 = game.player(p).counters.iter().filter(|(k, _)| k == "poison").map(|(_, n)| *n).sum();
    if poison < 9 {
        add_poison(game, p, 9 - poison);
    }
    Some(())
}
