//! `CantGainLifeAllEffect(Duration.EndOfTurn, TargetController.OPPONENT)`: "Your opponents can't gain life this turn."
//! (Roiling Vortex's activated ability). Java sets `canGainLife` false for each opponent until the cleanup; here a
//! gain-life replacement that stops the event, lasting as long (a static handler resolved as a one-shot effect).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("CantGainLifeAllEffect", &[EventType::GainLife], applies, replace)
        .args(&["one_of:Duration.EndOfTurn", "one_of:TargetController.OPPONENT"])];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.opponent(ev.player)
}

fn replace(_: &mut Game, _: &StaticCx, _: &mut Event) -> bool {
    true
}
