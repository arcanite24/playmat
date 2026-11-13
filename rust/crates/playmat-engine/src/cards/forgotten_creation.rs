//! Forgotten Creation: "At the beginning of your upkeep, you may discard all the cards in your hand. If you do, draw
//! that many cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ForgottenCreation.ForgottenCreationEffect", apply)];

/// As Java: draws as many as were in hand, however many were discarded
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.player(you).hand.len();
    if n > 0 {
        discard(game, d, you, n as i32);
        draw(game, you, n);
    }
    Some(())
}
