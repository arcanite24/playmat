//! Nine Lives: "Hexproof. If a source would deal damage to you, prevent that damage and put an incarnation counter
//! on Nine Lives. When there are nine or more incarnation counters on Nine Lives, exile it. When Nine Lives leaves
//! the battlefield, you lose the game."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "NineLives.NineLivesPreventionEffect",
    &[EventType::DamagePlayer],
    applies,
    replace,
)];

/// damage that can be prevented, to you
fn applies(_: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false) && ev.player == cx.controller
}

/// an incarnation counter on Nine Lives while it's on the battlefield, then all of the damage prevented
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if game.obj(cx.source).zone == Zone::Battlefield {
        put_counters(game, cx.source, "incarnation", 1, cx.controller);
    }
    cx.prevent(ev, i32::MAX);
    false
}
