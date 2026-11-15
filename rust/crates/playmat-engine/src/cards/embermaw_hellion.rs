//! Embermaw Hellion: "Trample. If another red source you control would deal damage to a permanent or player, it
//! deals that much damage plus 1 to that permanent or player instead."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EmbermawHellion.EmbermawHellionEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage from another red source you control (as it last was on the battlefield)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(s) = ev.source else { return false };
    s != cx.source && game.obj(s).controller == cx.controller && game.colors(s).contains(&Color::R)
}

/// `CardUtil.overflowInc(amount, 1)`
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
