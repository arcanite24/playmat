//! Calamity Bearer: "If a Giant source you control would deal damage to a permanent or player, it deals double that
//! damage to that permanent or player instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "CalamityBearer.CalamityBearerEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    double,
)];

/// some damage from a source you control that's a Giant: the permanent (or its last known information, gone to a
/// graveyard or exile), else the object itself (a spell)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    if ev.amount <= 0 || game.obj(s).controller != cx.controller {
        return false;
    }
    let o = game.obj(s);
    match o.zone {
        Zone::Graveyard | Zone::Exile => o.lki_def.as_ref().unwrap_or(&o.def).has_subtype("Giant"),
        _ => game.has_subtype(s, "Giant"),
    }
}

/// `CardUtil.overflowMultiply(amount, 2)`
fn double(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_mul(2);
    false
}
