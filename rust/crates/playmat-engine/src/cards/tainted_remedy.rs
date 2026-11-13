//! Tainted Remedy: "If an opponent would gain life, that player loses that much life instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "TaintedRemedy.TaintedRemedyReplacementEffect",
    &[EventType::GainLife],
    applies,
    replace,
)];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.opponent(ev.player)
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    // `opponent.loseLife(event.getAmount(), game, source, false)`
    lose_life_by(game, ev.player, ev.amount, Some(cx.source), false);
    true
}
