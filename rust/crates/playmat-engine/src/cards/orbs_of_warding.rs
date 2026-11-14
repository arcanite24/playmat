//! Orbs of Warding: "If a creature would deal damage to you, prevent 1 of that damage."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::replacement("OrbsOfWarding.OrbsOfWardingEffect", &[EventType::DamagePlayer], applies, replace)];

/// `DAMAGE_PLAYER` to you from a creature (`getPermanentOrLKIBattlefield`), damage that can be prevented
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    let permanent = game.obj(s).zone == Zone::Battlefield || game.lookback.contains(&s);
    ev.player == cx.controller && permanent && game.has_type(s, CardType::Creature) && cx.preventable(ev, false)
}

/// `PreventionEffectImpl(WhileOnBattlefield, 1, false, false)`: 1 of each such damage, not used up
fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, 1);
    false
}
