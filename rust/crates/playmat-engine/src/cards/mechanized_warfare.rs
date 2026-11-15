//! Mechanized Warfare: "If a red or artifact source you control would deal damage to an opponent or a permanent an
//! opponent controls, it deals that much damage plus 1 instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MechanizedWarfare.MechanizedWarfareEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage to an opponent (or a permanent an opponent controls) from a red or artifact source you control
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    ev.player != cx.controller
        && game.obj(s).controller == cx.controller
        && ev.amount > 0
        && (game.colors(s).contains(&crate::card::Color::R) || game.has_type(s, CardType::Artifact))
}

/// `CardUtil.overflowInc(amount, 1)`
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
