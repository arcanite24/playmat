//! Insult // Injury (Insult): "If a source you control would deal damage this turn, it deals double that damage
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "InsultInjury.InsultDoubleDamageEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// `game.getControllerId(sourceId)` is you
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.source.is_some_and(|s| game.obj(s).controller == cx.controller)
}

/// `CardUtil.overflowMultiply(amount, 2)`
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_mul(2);
    false
}
