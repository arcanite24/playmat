//! Wildgrowth Archaic: "Trample, reach. Converge — This creature enters with a +1/+1 counter on it for each color of
//! mana spent to cast it. Whenever you cast a creature spell, that creature enters with X additional +1/+1 counters
//! on it, where X is the number of colors of mana spent to cast it."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "WildgrowthArchaic.WildgrowthArchaicEffect",
    &[EventType::EntersBattlefield],
    applies,
    replace,
)];

/// the spell the trigger points at (`SetTargetPointer.SPELL`) entering as a permanent
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    matches!(cx.targets.first(), Some((Target::Obj(s), zcc)) if *s == id && game.obj(id).zcc <= zcc + 1)
}

/// as many +1/+1 counters, put by you, as colors of mana were spent to cast it
fn replace(game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    if let Some(id) = ev.target {
        let colors = game.obj(id).spent_colors.iter().filter(|n| **n > 0).count() as i32;
        if colors > 0 {
            add_counters_by(game, id, "+1/+1", colors, Some(cx.controller));
        }
    }
    false
}
