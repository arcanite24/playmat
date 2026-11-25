//! `SkipNextPlayerUntapStepEffect(text)`: "that player skips their next untap step" (Dovin, Architect of Law's
//! -9): the target player, or the controller when there's no target, gets a `TurnMod` that skips their next untap
//! step.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SkipNextPlayerUntapStepEffect", apply).args(&["text"])];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let player = cx.player(0).unwrap_or_else(|| cx.controller());
    game.skip_untap_steps.push(player);
    Some(())
}
