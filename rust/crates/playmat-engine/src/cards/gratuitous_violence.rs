//! Gratuitous Violence: "If a creature you control would deal damage to a permanent or player, it deals double that
//! damage instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "GratuitousViolence.GratuitousViolenceReplacementEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    double,
)];

/// the source a creature you control (`getPermanentOrLKIBattlefield`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.source.is_some_and(|s| permanent_or_lki_is(game, s, CardType::Creature) && game.obj(s).controller == cx.controller)
}

/// the damage source a permanent of that type, or, gone to a graveyard or exile, last known as one: Java keeps the
/// battlefield's last known information through the step, so a dies trigger's damage still counts
pub(super) fn permanent_or_lki_is(game: &Game, s: ObjId, ty: CardType) -> bool {
    let o = game.obj(s);
    match o.zone {
        Zone::Battlefield => game.has_type(s, ty),
        Zone::Graveyard | Zone::Exile => o.lki_def.as_ref().unwrap_or(&o.def).is(ty),
        _ => false,
    }
}

/// `CardUtil.overflowMultiply(amount, 2)`
fn double(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_mul(2);
    false
}
