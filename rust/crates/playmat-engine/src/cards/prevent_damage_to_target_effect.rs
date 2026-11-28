//! `PreventDamageToTargetEffect`: "Until your next turn, prevent all damage that would be dealt to target permanent
//! an opponent controls" (Dovin, Hand of Control), "prevent the next 3 damage that would be dealt to any target this
//! turn"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventDamageToTargetEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["text", "text", "text"])];

/// (onlyCombat, amountToPrevent): `(duration)`, `(duration, onlyCombat)`, `(duration, amount)` or
/// `(duration, amount, onlyCombat)`
fn shape(cx: &StaticCx) -> (bool, i32) {
    match cx.args.len() {
        2 => match java_int(&cx.args[1]) {
            Some(n) => (false, n),
            None => (cx.arg_bool(1), i32::MAX),
        },
        3 => (cx.arg_bool(2), java_int(&cx.args[1]).unwrap_or(0)),
        _ => (false, i32::MAX),
    }
}

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let (combat, n) = shape(cx);
    let to = match ev.kind {
        EventType::DamagePermanent => (0..cx.targets.len()).any(|i| ev.target.is_some() && cx.target_obj(game, i) == ev.target),
        _ => (0..cx.targets.len()).any(|i| cx.target_player(i) == Some(ev.player)),
    };
    to && cx.preventable(ev, combat) && cx.left(game, n) > 0
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let (_, n) = shape(cx);
    let left = cx.left(game, n);
    let prevented = cx.prevent(ev, left);
    if left != i32::MAX {
        cx.set_left(game, left - prevented);
    }
    false
}
