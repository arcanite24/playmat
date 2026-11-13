//! `PreventAllDamageByAttachedEffect`: "Prevent all combat damage that would be dealt by enchanted creature."
//! (Candletrap; all damage: Armament of Nyx's)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventAllDamageByAttachedEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["while_on_battlefield", "text", "text"])];

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, cx.arg_bool(2)) && ev.source.is_some() && game.attached(cx.source) == ev.source
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
