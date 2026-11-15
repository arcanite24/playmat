//! Servant of the Scale: "When Servant of the Scale dies, put X +1/+1 counters on target creature you control, where
//! X is the number of +1/+1 counters on Servant of the Scale."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ServantOfTheScale.ServantOfTheScaleEffect", apply)];

/// its +1/+1 counters as it last was on the battlefield
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let s = game.obj(cx.source_id());
    let counters = if s.zone == Zone::Battlefield { &s.counters } else { &s.lki_counters };
    let n: i32 = counters.iter().filter(|(k, _)| k == "+1/+1").map(|(_, n)| *n).sum();
    if n > 0 {
        if let Some(o) = cx.obj(0) {
            put_counters(game, o, "+1/+1", n, cx.controller());
        }
    }
    Some(())
}
