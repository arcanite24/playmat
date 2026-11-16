//! Anti-Venom, Horrifying Healer: "If damage would be dealt to Anti-Venom, prevent that damage and put that many
//! +1/+1 counters on him."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "AntiVenomHorrifyingHealer.AntiVenomHorrifyingHealerEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// preventable damage to Anti-Venom itself
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false) && ev.target == Some(cx.source)
}

/// that many +1/+1 counters first, then the damage prevented
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone == Zone::Battlefield {
        add_counters_by(game, cx.source, "+1/+1", ev.amount, Some(cx.controller));
    }
    cx.prevent(ev, i32::MAX);
    false
}
