//! Goldnight Castigator: "Flying, haste. If a source would deal damage to you, it deals double that damage to you
//! instead. If a source would deal damage to Goldnight Castigator, it deals double that damage to Goldnight
//! Castigator instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "GoldnightCastigator.GoldnightCastigatorDoubleDamageEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// Java's `applies` is true for every damage event; `replaceEvent` reads the target
fn applies(_: &Game, _: &StaticCx, _: &Event) -> bool {
    true
}

/// damage to you or to this permanent doubled (`CardUtil.overflowMultiply`)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let doubles = match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target == Some(cx.source) && game.obj(cx.source).zone == Zone::Battlefield,
    };
    if doubles {
        ev.amount = ev.amount.saturating_mul(2);
    }
    false
}
