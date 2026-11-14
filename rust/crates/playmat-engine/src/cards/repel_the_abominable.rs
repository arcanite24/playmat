//! Repel the Abominable: "Prevent all damage that would be dealt this turn by non-Human sources."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "RepelTheAbominable.RepelTheAbominablePreventionEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// Any damage this turn whose source isn't a Human (`hasSubtype(SubType.HUMAN)`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    cx.preventable(ev, false) && ev.source.is_some_and(|s| !game.has_subtype(s, "Human"))
}

fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
