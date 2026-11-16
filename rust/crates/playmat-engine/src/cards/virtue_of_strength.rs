//! Virtue of Strength // Garden's Bounty: "If you tap a basic land for mana, it produces three times as much of that
//! mana instead." // "Return up to three target permanent cards from your graveyard to your hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "VirtueOfStrength.VirtueOfStrengthReplacementEffect",
    &[EventType::TappedForMana],
    applies,
    replace,
)];

/// `TAPPED_FOR_MANA` of a basic land, by you
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    ev.player == cx.controller
        && ev.target.is_some_and(|l| {
            game.has_type(l, CardType::Land) && game.obj(l).def.supertypes.iter().any(|s| s == "BASIC")
        })
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
