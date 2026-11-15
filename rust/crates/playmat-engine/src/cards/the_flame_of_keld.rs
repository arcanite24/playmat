//! The Flame of Keld: "I — Discard your hand. II — Draw two cards. III — If a red source you control would deal
//! damage to a permanent or player this turn, it deals that much damage plus 2 to that permanent or player instead."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "TheFlameOfKeld.TheFlameOfKeldDamageEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    plus_two,
)];

/// damage from a red source you control (as it is, or as it last was on the battlefield) other than the Saga
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    game.obj(src).controller == cx.controller && src != cx.source && game.colors(src).contains(&Color::R)
}

/// `CardUtil.overflowInc(amount, 2)`
fn plus_two(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(2);
    false
}
