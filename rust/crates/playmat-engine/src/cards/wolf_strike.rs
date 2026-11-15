//! Wolf Strike: "Target creature you control gets +2/+0 until end of turn if it's night. Then it deals damage equal to
//! its power to target creature you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("WolfStrike.WolfStikeEffect", apply)
    .effects(&["new BoostTargetEffect(2, 0, Duration.EndOfTurn)"])];

/// at night (`checkDayNight(false)`), +2/+0 on the first target
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if game.day == Some(false) {
        cx.run(game, d, cx.effects);
    }
    (!d.stopped()).then_some(())
}
