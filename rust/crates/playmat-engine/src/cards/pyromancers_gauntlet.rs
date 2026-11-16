//! Pyromancer's Gauntlet: "If a red instant or sorcery spell you control or a red planeswalker you control would deal
//! damage to a permanent or player, it deals that much damage plus 2 to that permanent or player instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PyromancersGauntlet.PyromancersGauntletReplacementEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    plus_two,
)];

/// Java's `applies` checks no color: an instant or sorcery spell you control, or a planeswalker you control
/// (`getPermanentOrLKIBattlefield`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    let o = game.obj(s);
    match o.zone {
        Zone::Stack => {
            o.controller == cx.controller && (game.has_type(s, CardType::Instant) || game.has_type(s, CardType::Sorcery))
        }
        _ => o.controller == cx.controller && gratuitous_violence::permanent_or_lki_is(game, s, CardType::Planeswalker),
    }
}

/// `CardUtil.overflowInc(amount, 2)`
fn plus_two(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(2);
    false
}
