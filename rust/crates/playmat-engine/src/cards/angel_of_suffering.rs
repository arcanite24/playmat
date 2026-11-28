//! Angel of Suffering: "If damage would be dealt to you, prevent that damage and mill twice that many cards."

use super::*;
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "AngelOfSuffering.AngelOfSufferingEffect",
    &[EventType::DamagePlayer],
    applies,
    replace,
)];

/// `PreventionEffectImpl.applies` (damage that can be prevented), to you
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false) && ev.kind == EventType::DamagePlayer && ev.player == cx.controller
}

/// twice the damage as it would have been, then the damage prevented and that many cards milled (`millCards`)
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    let n = (ev.amount * 2).max(0) as usize;
    cx.prevent(ev, i32::MAX);
    let top: Vec<ObjId> = game.player(cx.controller).library.iter().take(n).copied().collect();
    note(game, EventKind::Milled, Some(cx.controller), None, None, top.len() as i32);
    for c in top {
        game.move_to(c, Zone::Graveyard);
    }
    false
}
