//! `MaximumHandSizeControllerEffect(Integer.MAX_VALUE, Duration.EndOfGame, SET)` as it resolves: "you have no
//! maximum hand size for the rest of the game" (Sea Gate Restoration)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MaximumHandSizeControllerEffect", apply).args(&[
    "one_of:Integer.MAX_VALUE",
    "one_of:Duration.EndOfGame",
    "one_of:MaximumHandSizeControllerEffect.HandSizeModification.SET|HandSizeModification.SET",
])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    game.player_mut(cx.controller()).no_max_hand_size = true;
    Some(())
}
