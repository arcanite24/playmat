//! Jaya, Venerated Firemage: "If another red source you control would deal damage to a permanent or player, it deals
//! that much damage plus 1 to that permanent or player instead."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "JayaVeneratedFiremage.JayaVeneratedFiremageEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// a red source other than Jaya, controlled by Jaya's controller (as it last was on the battlefield)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(src) = ev.source else { return false };
    src != cx.source && game.obj(src).controller == cx.controller && game.colors(src).contains(&Color::R)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
