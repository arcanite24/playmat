//! Throw from the Saddle: "Target creature you control gets +1/+1 until end of turn. Put a +1/+1 counter on it
//! instead if it's a Mount. Then it deals damage equal to its power to target creature you don't control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ThrowFromTheSaddle.ThrowFromTheSaddleEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.has_subtype(o, "Mount") {
        put_counters(game, o, "+1/+1", 1, cx.controller());
    } else {
        boost(game, o, 1, 1, Duration::EndOfTurn);
    }
    Some(())
}
