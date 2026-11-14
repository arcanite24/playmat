//! `PreventDamageByTargetEffect`: "Prevent all damage target instant or sorcery spell would deal this turn."
//! (Dromoka's Command), "until your next turn, prevent all damage target permanent would deal" (Gideon of the
//! Trials, Dovin, Hand of Control)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventDamageByTargetEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["text", "text", "text"])];

/// (onlyCombat, amountToPrevent): `(duration, onlyCombat)` or `(duration, amount, onlyCombat)`
fn shape(cx: &StaticCx) -> (bool, i32) {
    match cx.args.len() {
        3 => (cx.arg_bool(2), java_int(&cx.args[1]).unwrap_or(0)),
        _ => (cx.arg_bool(1), i32::MAX),
    }
}

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let (combat, n) = shape(cx);
    cx.preventable(ev, combat)
        && cx.left(game, n) > 0
        && ev.source.is_some()
        && (0..cx.targets.len()).any(|i| cx.target_obj(game, i) == ev.source)
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
