//! Might of Murasa: "Kicker {2}{G}. Target creature gets +3/+3 until end of turn. If this spell was kicked, that
//! creature gets +5/+5 until end of turn instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MightOfMurasa.MightOfMurasaEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let n = if cx.item.kicked { 5 } else { 3 };
    boost(game, o, n, n, Duration::EndOfTurn);
    Some(())
}
