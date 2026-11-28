//! Kiora, the Crashing Wave: "+1: Until your next turn, prevent all damage that would be dealt to and dealt by
//! target permanent an opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "KioraTheCrashingWave.KioraPreventionEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)
.until_your_next_turn()];

/// damage dealt by the target permanent, or to it (`PreventionEffectImpl`, not only combat damage)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(it) = cx.target_obj(game, 0) else { return false };
    let by = ev.source == Some(it);
    let to = ev.kind == EventType::DamagePermanent && ev.target == Some(it);
    (by || to) && cx.preventable(ev, false)
}

fn replace(_: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
