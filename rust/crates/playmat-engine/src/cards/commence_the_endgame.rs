//! Commence the Endgame: "Draw two cards, then amass Zombies X, where X is the number of cards in your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CommenceTheEndgame.CommenceTheEndgameEffect", apply)
    .effects(&["new AmassEffect(1, SubType.ZOMBIE)"])];

/// the hand counted after the draw (`AmassEffect.doAmass(hand.size(), ZOMBIE, …)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 2);
    let n = game.player(you).hand.len() as i32;
    let mut e = cx.effects[0].clone();
    if let EffectDef::AmassEffect { amount, .. } = &mut e {
        *amount = Amount::Fixed(n);
    }
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
