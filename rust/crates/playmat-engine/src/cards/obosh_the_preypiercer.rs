//! Obosh, the Preypiercer: "Companion — Your starting deck contains only cards with odd mana values and land cards.
//! If a source you control with an odd mana value would deal damage to a permanent or player, it deals double that
//! damage to that permanent or player instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "OboshThePreypiercer.OboshThePreypiercerEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// the damage's source has an odd mana value and is controlled by you
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    let o = game.obj(s);
    o.def.mana_value() % 2 == 1 && o.controller == cx.controller
}

/// the damage doubled (`CardUtil.overflowMultiply`)
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_mul(2);
    false
}
