//! Lithomancer's Focus: "Target creature gets +2/+2 until end of turn. Prevent all damage that would be dealt to that
//! creature this turn by colorless sources."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "LithomancersFocus.LithomancersFocusPreventDamageToTargetEffect",
    &[EventType::DamagePermanent],
    applies,
    replace,
)];

/// Damage to the spell's target from a colorless source (`getObject(sourceId).getColor().isColorless()`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false)
        && ev.target.is_some()
        && cx.target_obj(game, 0) == ev.target
        && ev.source.is_some_and(|s| game.colors(s).is_empty())
}

fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
