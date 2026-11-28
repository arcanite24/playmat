//! Djeru, With Eyes Open: "Vigilance. When Djeru enters, you may search your library for a planeswalker card, reveal
//! it, put it into your hand, then shuffle. If a source would deal damage to a planeswalker you control, prevent 1
//! of that damage."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "DjeruWithEyesOpen.DjeruWithEyesOpenPreventEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage that can be prevented, to a planeswalker you control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false)
        && ev.kind == EventType::DamagePermanent
        && ev.target.is_some_and(|t| {
            game.obj(t).controller == cx.controller && game.has_type(t, CardType::Planeswalker)
        })
}

/// 1 of it prevented, each time (not consumable)
fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, 1);
    false
}
