//! Eerie Interference: "Prevent all damage that would be dealt this turn to you and creatures you control by
//! creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "EerieInterference.EerieInterferenceEffect",
    &[EventType::DamagePlayer, EventType::DamagePermanent],
    applies,
    replace,
)];

/// damage to you, or to a creature you control, from a creature (as it last was on the battlefield)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    if !cx.preventable(ev, false) {
        return false;
    }
    let to_you = match ev.kind {
        EventType::DamagePlayer => ev.player == cx.controller,
        _ => ev.target.is_some_and(|t| {
            game.obj(t).controller == cx.controller && game.has_type(t, CardType::Creature)
        }),
    };
    to_you && ev.source.is_some_and(|s| game.has_type(s, CardType::Creature))
}

fn replace(_game: &mut Game, cx: &StaticCx, ev: &mut Event) -> bool {
    cx.prevent(ev, i32::MAX);
    false
}
