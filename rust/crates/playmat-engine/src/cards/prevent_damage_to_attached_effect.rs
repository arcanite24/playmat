//! `PreventDamageToAttachedEffect`: "If a source would deal damage to equipped creature, prevent 2 of that damage."
//! (Shield of the Realm; "prevent X of that damage, where X is the number of creatures you control": Shield of the
//! Avatar)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventDamageToAttachedEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["while_on_battlefield", "text", "text", "text"])];

/// (onlyCombat, amountToPrevent) of the constructor used: `(duration, type, combatOnly)`,
/// `(duration, type, amount, combatOnly)` or `(duration, type, combatOnly, CreaturesYouControlCount)`
fn shape(game: &Game, cx: &StaticCx) -> Option<(bool, i32)> {
    match cx.args.len() {
        3 => Some((cx.arg_bool(2), i32::MAX)),
        4 if cx.args[3].trim() == "CreaturesYouControlCount.PLURAL"
            || cx.args[3].trim() == "CreaturesYouControlCount.SINGULAR" =>
        {
            let n = game
                .battlefield
                .iter()
                .filter(|o| game.obj(**o).controller == cx.controller && game.has_type(**o, CardType::Creature))
                .count();
            Some((cx.arg_bool(2), n as i32))
        }
        4 => Some((cx.arg_bool(3), java_int(&cx.args[2])?)),
        _ => None,
    }
}

fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some((combat, _)) = shape(game, cx) else { return false };
    cx.preventable(ev, combat) && ev.target.is_some() && game.attached(cx.source) == ev.target
}

fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some((_, n)) = shape(game, cx) {
        cx.prevent(ev, n);
    }
    false
}
