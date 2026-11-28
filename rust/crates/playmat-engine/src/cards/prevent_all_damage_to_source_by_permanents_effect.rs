//! `PreventAllDamageToSourceByPermanentsEffect(filter, onlyCombat)` (Armored Transport): "Prevent all combat damage
//! that would be dealt to {this} by creatures blocking it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventAllDamageToSourceByPermanentsEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["filter", "bool"])];

/// `PreventAllDamageByAllPermanentsEffect.applies`: preventable damage (combat damage when `onlyCombat`) of more
/// than 0 from a permanent of the filter, dealt to the source
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, cx.arg_bool(1))
        && ev.amount > 0
        && ev.target == Some(cx.source)
        && ev.source.is_some_and(|s| game.obj(s).zone == Zone::Battlefield && cx.matches(game, 0, s))
}

/// all of it
fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
