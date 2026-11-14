//! `CantAttackBlockTargetEffect(Duration.EndOfTurn)`: "target creature can't attack or block this turn" (Endbringer):
//! the engine's "can't attack" and "can't block" until end of turn, on the target.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CantAttackBlockTargetEffect", apply)
    .args(&["one_of:Duration.EndOfTurn"])
    .effects(&["new CantAttackTargetEffect(Duration.EndOfTurn)", "new CantBlockTargetEffect(Duration.EndOfTurn)"])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..2]);
    Some(())
}
