//! Thrun, Breaker of Silence: "Thrun can't be the target of nongreen spells your opponents control or abilities from
//! nongreen sources your opponents control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("ThrunBreakerOfSilence.ThrunBreakerOfSilenceEffect", Rule::CantBeTargeted(nongreen_opponent))];

/// `TARGET` of Thrun itself by the stack object of `src` (`getStackObject`, the topmost: a spell, or an ability of
/// that source, which both engines put on the stack before its targets) that an opponent controls, when the spell,
/// or the ability's source, isn't green. With no stack object (the playable check of a spell in hand) nothing stops
/// it.
fn nongreen_opponent(game: &Game, cx: &StaticCx, _seat: Seat, id: ObjId, src: ObjId) -> bool {
    let Some(item) = game.stack.iter().rev().find(|i| i.card == src) else { return false };
    id == cx.source && item.controller != cx.controller && !game.colors(src).contains(&Color::G)
}
