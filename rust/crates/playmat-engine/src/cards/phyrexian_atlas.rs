//! Phyrexian Atlas: "Corrupted — Whenever Phyrexian Atlas becomes tapped, each opponent who has three or more poison
//! counters loses 1 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PhyrexianAtlas.PhyrexianAtlasEffect", apply)];

/// each opponent in turn order
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> =
        (1..game.seats()).map(|i| (you + i) % game.seats()).filter(|p| !game.player(*p).lost).collect();
    for p in opponents {
        let poison: i32 = game.player(p).counters.iter().filter(|(k, _)| k == "poison").map(|(_, c)| *c).sum();
        if poison >= 3 {
            lose_life(game, p, 1);
        }
    }
    Some(())
}
