//! Protection of the Hekma: "If a source an opponent controls would deal damage to you, prevent 1 of that damage."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "ProtectionOfTheHekma.ProtectionOfTheHekmaEffect",
    &[EventType::DamagePlayer],
    applies,
    replace,
)];

/// `DAMAGE_PLAYER` to you from a source an opponent controls (`game.getControllerId`), damage that can be prevented
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    ev.player == cx.controller && cx.opponent(game.obj(src).controller) && cx.preventable(ev, false)
}

/// `PreventionEffectImpl(…, 1, false, false)`: 1 of each such damage, not used up
fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, 1);
    false
}
