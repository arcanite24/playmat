//! Tavern Swindler: "{T}, Pay 3 life: Flip a coin. If you win the flip, you gain 6 life."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TavernSwindler.TavernSwindlerEffect", apply)];

/// `controller.flipCoin(source, game, true)`, then 6 life on a win
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let won = vec![EffectDef::GainLifeEffect { amount: crate::card::Amount::Fixed(6) }];
    cx.run(game, d, &[EffectDef::FlipCoinEffect { won, lost: Vec::new() }]);
    (!d.stopped()).then_some(())
}
