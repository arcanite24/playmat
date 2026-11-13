//! `PreventAllDamageToPlayersEffect`: "Prevent all combat damage that would be dealt to players this turn."
//! (Commencement of Festivities, Defend the Hearth)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventAllDamageToPlayersEffect",
    &[EventType::DamagePlayer],
    applies,
    replace,
)
.args(&["text", "text"])];

fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, cx.arg_bool(1))
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
