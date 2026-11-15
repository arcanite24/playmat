//! Stormwild Capridor: "Flying. If noncombat damage would be dealt to Stormwild Capridor, prevent that damage. Put a
//! +1/+1 counter on Stormwild Capridor for each 1 damage prevented this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "StormwildCapridor.StormwildCapridorEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// preventable noncombat damage to it
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.target == Some(cx.source) && cx.preventable(ev, false) && !ev.combat
}

/// all of it prevented, then that many +1/+1 counters put on it by you, while it's on the battlefield
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let prevented = cx.prevent(ev, i32::MAX);
    if prevented > 0 && game.obj(cx.source).zone == Zone::Battlefield {
        put_counters(game, cx.source, "+1/+1", prevented, cx.controller);
    }
    false
}
