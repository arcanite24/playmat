//! `PreventAllDamageToSourceEffect(Duration.WhileOnBattlefield)` in a static ability: "Prevent all damage that would
//! be dealt to {this}." (Black Panther, Hope Enduring). The end-of-turn one a spell or ability makes is the engine's
//! own effect.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "PreventAllDamageToSourceEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)
.args(&["text"])];

/// `PreventionEffectImpl.applies` (damage that can be prevented) to the source itself
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false) && ev.target == Some(cx.source)
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
