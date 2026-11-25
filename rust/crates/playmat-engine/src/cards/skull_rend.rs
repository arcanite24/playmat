//! Skull Rend: "Skull Rend deals 2 damage to each opponent. Those players each discard two cards at random."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkullRend.SkullRendEffect", apply)];

/// 2 damage to each opponent, then each of them discards two cards at random (`discard(2, true, …)`)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let opponents: Vec<Seat> = (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost).collect();
    for p in &opponents {
        deal_damage(game, cx.source_id(), Target::Player(*p), 2, false);
    }
    for p in opponents {
        discard_at_random(game, p, 2);
    }
    Some(())
}
