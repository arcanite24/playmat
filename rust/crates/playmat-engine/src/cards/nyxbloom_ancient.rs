//! Nyxbloom Ancient: "Trample. If you tap a permanent for mana, it produces three times as much of that mana
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "NyxbloomAncient.NyxbloomAncientReplacementEffect",
    &[EventType::TappedForMana],
    applies,
    replace,
)];

/// `TAPPED_FOR_MANA` by you (`source.isControlledBy(event.getPlayerId())`)
fn applies(_game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
}

/// each kind of the mana it makes, three times as much (`CardUtil.overflowMultiply`)
fn replace(_game: &mut Game, _cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(m) = ev.mana.as_mut() {
        for n in m.colored.iter_mut() {
            *n = n.saturating_mul(3);
        }
        m.colorless = m.colorless.saturating_mul(3);
    }
    false
}
