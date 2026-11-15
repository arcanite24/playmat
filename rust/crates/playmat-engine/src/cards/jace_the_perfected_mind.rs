//! Jace, the Perfected Mind: "−2: Target player mills three cards. Then if a graveyard has twenty or more cards in it,
//! you draw three cards. Otherwise, you draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JaceThePerfectedMind.JaceThePerfectedMindEffect", apply)];

/// the mill, then any graveyard of a player in range (all of them, in a two-player game) with twenty or more
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.player(0)?;
    cx.run(game, d, &[EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(3) }]);
    let big = (0..game.seats()).any(|p| game.player(p).graveyard.len() >= 20);
    draw(game, cx.controller(), if big { 3 } else { 1 });
    (!d.stopped()).then_some(())
}
