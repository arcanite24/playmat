//! Far Fortune, End Boss: "Max speed — If a source you control would deal damage to an opponent or a permanent an
//! opponent controls, it deals that much damage plus 1 instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "FarFortuneEndBoss.FarFortuneEndBossEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// the source's controller is you, and the damaged player (or the damaged permanent's controller) an opponent
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.source.is_some_and(|s| game.obj(s).controller == cx.controller) && ev.player != cx.controller
}

/// `CardUtil.overflowInc(amount, 1)`
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
