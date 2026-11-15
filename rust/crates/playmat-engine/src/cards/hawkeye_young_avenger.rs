//! Hawkeye, Young Avenger: "Reach. If a source you control would deal noncombat damage to an opponent or a permanent
//! an opponent controls, instead it deals that much damage plus X, where X is Hawkeye's power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "HawkeyeYoungAvenger.HawkeyeYoungAvengerEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// noncombat damage of at least 1, from a source you control, to an opponent or a permanent an opponent controls
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if ev.combat || ev.amount < 1 {
        return false;
    }
    let Some(s) = ev.source else { return false };
    game.obj(s).controller == cx.controller && cx.opponent(ev.player)
}

/// plus Hawkeye's power, not negative (`SourcePermanentPowerValue.NOT_NEGATIVE`, `CardUtil.overflowInc`)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let x = game.power_toughness(cx.source).0.max(0);
    ev.amount = ev.amount.saturating_add(x);
    false
}
