//! Rem Karolus, Stalwart Slayer: "Flying, haste. If a spell would deal damage to you or another permanent you
//! control, prevent that damage. If a spell would deal damage to an opponent or a permanent an opponent controls, it
//! deals that much damage plus 1 instead."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::replacement(
        "RemKarolusStalwartSlayer.RemKarolusStalwartSlayerPreventionEffect",
        &[EventType::DamagePlayer, EventType::DamagePermanent],
        to_yours,
        prevent_all,
    ),
    Handler::replacement(
        "RemKarolusStalwartSlayer.RemKarolusStalwartSlayerReplacementEffect",
        &[EventType::DamagePlayer, EventType::DamagePermanent],
        to_opponents,
        plus_one,
    ),
];

/// the source is a spell (`getStackObject` of the source: an object on the stack, abilities' sources aren't)
fn from_spell(game: &Game, ev: &Event) -> bool {
    ev.source.is_some_and(|s| game.obj(s).zone == Zone::Stack)
}

/// preventable damage from a spell to you, or to a permanent you control other than Rem Karolus
fn to_yours(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.preventable(ev, false) {
        return false;
    }
    let hit = match ev.kind {
        EventType::DamagePermanent => ev.target.is_some_and(|t| t != cx.source && game.obj(t).controller == cx.controller),
        _ => ev.player == cx.controller,
    };
    hit && from_spell(game, ev)
}

fn prevent_all(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}

/// damage from a spell to an opponent, or to a permanent an opponent controls
fn to_opponents(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let hit = match ev.kind {
        EventType::DamagePermanent => ev.target.is_some_and(|t| game.obj(t).controller != cx.controller),
        _ => ev.player != cx.controller,
    };
    hit && from_spell(game, ev)
}

/// `CardUtil.overflowInc(amount, 1)`
fn plus_one(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.amount = ev.amount.saturating_add(1);
    false
}
