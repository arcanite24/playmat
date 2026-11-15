//! Psychic Spiral: "Shuffle all cards from your graveyard into your library. Target player mills that many cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PsychicSpiral.PsychicSpiralEffect", apply)];

/// the graveyard's size before the shuffle, then the mill when it was any
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.player(0)?;
    let n = game.player(cx.controller()).graveyard.len() as i32;
    cx.run(game, d, &[EffectDef::ShuffleYourGraveyardIntoLibraryEffect]);
    if n > 0 {
        cx.run(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(n) }]);
    }
    (!d.stopped()).then_some(())
}
