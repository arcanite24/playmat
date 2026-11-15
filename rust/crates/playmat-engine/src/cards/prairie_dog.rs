//! Prairie Dog: "Lifelink. At the beginning of your end step, if you haven't cast a spell from your hand this turn,
//! put a +1/+1 counter on Prairie Dog. {4}{W}: Until end of turn, if you would put one or more +1/+1 counters on a
//! creature you control, put that many plus one +1/+1 counters on it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PrairieDog.PrairieDogReplacementEffect",
    &[EventType::AddCounters],
    applies,
    plus_one,
)];

/// some +1/+1 counters you'd put on a creature you control (on the battlefield)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(o) = ev.target else { return false };
    ev.amount > 0
        && ev.player == cx.controller
        && ev.counter.as_deref() == Some("+1/+1")
        && game.obj(o).zone == Zone::Battlefield
        && game.obj(o).controller == cx.controller
        && game.has_type(o, CardType::Creature)
}

/// `CardUtil.overflowInc(amount, 1)`
fn plus_one(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
