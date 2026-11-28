//! `GainDoubleLifeReplacementEffect`: "If you would gain life, you gain twice that much life instead." (Alhammarret's
//! Archive, The Wind Crystal)

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("GainDoubleLifeReplacementEffect", &[EventType::GainLife], applies, replace)];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_mul(2);
    false
}
