//! `PreventAllDamageFromChosenSourceToYouEffect(duration)`: "Prevent all damage that would be dealt to you this turn
//! by a source of your choice" (Consulate Surveillance). The filter and combat-only forms aren't read.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventAllDamageFromChosenSourceToYouEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["duration"])
.on_start(choose_source)];

/// damage to you from the chosen source, while it's still the object chosen (`targetSource.isLegal`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some((Target::Obj(src), zcc)) = cx.targets.first() else { return false };
    cx.preventable(ev, false)
        && ev.kind == EventType::DamagePlayer
        && ev.player == cx.controller
        && ev.source == Some(*src)
        && game.obj(*src).zcc == *zcc
}

/// all of it
fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
