//! Ajani Steadfast: "+1: Until end of turn, up to one target creature gets +1/+1 and gains first strike, vigilance,
//! and lifelink. −2: Put a +1/+1 counter on each creature you control and a loyalty counter on each other
//! planeswalker you control. −7: You get an emblem with "If a source would deal damage to you or a planeswalker you
//! control, prevent all but 1 of that damage.""

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "AjaniSteadfastEmblem.AjaniSteadfastPreventEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage that can be prevented, to you or to a planeswalker you control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.preventable(ev, false) {
        return false;
    }
    match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target.is_some_and(|t| {
            game.obj(t).zone == Zone::Battlefield
                && game.obj(t).controller == cx.controller
                && game.has_type(t, CardType::Planeswalker)
        }),
    }
}

/// all but 1 of it prevented, when it's more than 1 (not consumable)
fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if ev.amount > 1 {
        cx.prevent(ev, ev.amount - 1);
    }
    false
}
