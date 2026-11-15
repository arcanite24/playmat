//! Collective Inferno: "Double all damage that sources you control of the chosen type would deal."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "CollectiveInferno.CollectiveInfernoEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// a source you control of the creature type it chose
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    let Some(t) = game.obj(cx.source).chosen_type.as_deref() else { return false };
    game.obj(s).controller == cx.controller && game.has_subtype(s, t)
}

/// `CardUtil.overflowMultiply(amount, 2)`
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_mul(2);
    false
}
