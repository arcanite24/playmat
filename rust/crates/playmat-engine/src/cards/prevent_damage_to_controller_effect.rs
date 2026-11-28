//! `PreventDamageToControllerEffect`: "Prevent all damage that would be dealt to you this turn." (Riot Control),
//! "the next 1 damage" (Security Blockade), "all combat damage" (Druid's Deliverance), "until your next turn"
//! (Morningtide's Light)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventDamageToControllerEffect",
    &[EventType::DamagePlayer],
    applies,
    replace,
)
.args(&["text", "text", "text", "text"])];

/// (onlyCombat, consumable, amountToPrevent) of the constructor used
fn shape(cx: &StaticCx) -> (bool, bool, i32) {
    match cx.args.len() {
        2 => (false, true, java_int(&cx.args[1]).unwrap_or(0)),
        4 => (cx.arg_bool(1), cx.arg_bool(2), java_int(&cx.args[3]).unwrap_or(0)),
        _ => (false, false, i32::MAX),
    }
}

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let (combat, _, n) = shape(cx);
    cx.preventable(ev, combat) && ev.player == cx.controller && cx.left(game, n) > 0
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let (_, consumable, n) = shape(cx);
    let left = cx.left(game, n);
    let prevented = cx.prevent(ev, left);
    if consumable && left != i32::MAX {
        cx.set_left(game, left - prevented);
    }
    false
}
