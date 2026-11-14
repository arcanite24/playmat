//! Rookie Mistake: "Until end of turn, target creature gets +0/+2 and another target creature gets -2/-0."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RookieMistake.RookieMistakeEffect", apply).effects(&[
    "new BoostTargetEffect(0, 2, Duration.EndOfTurn)",
    "new BoostTargetEffect(-2, 0, Duration.EndOfTurn)",
])];

/// each boost on its own target, when it's still there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for i in 0..2 {
        if let Some(o) = cx.obj(i).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
            cx.run_on(game, d, &cx.effects[i..i + 1], vec![Target::Obj(o)]);
            if d.stopped() {
                return None;
            }
        }
    }
    Some(())
}
