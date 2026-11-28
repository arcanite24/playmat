//! Lazotep Plating: "Amass Zombies 1. You and permanents you control gain hexproof until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LazotepPlating.GainAbilityControllerEffect", apply)
    .args(&["one_of:HexproofAbility.getInstance()", "one_of:Duration.EndOfTurn"])];

/// `GainAbilityControllerEffect(HexproofAbility.getInstance(), Duration.EndOfTurn)`: you have hexproof the rest of
/// this turn
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let turn = game.turn;
    game.player_mut(cx.controller()).hexproof_turn = Some(turn);
    Some(())
}
