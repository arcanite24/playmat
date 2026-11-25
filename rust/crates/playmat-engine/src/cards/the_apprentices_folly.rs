//! The Apprentice's Folly: "I, II — Choose target nontoken creature you control that doesn't have the same name as a
//! token you control. Create a token that's a copy of it, except it isn't legendary, is a Reflection in addition to
//! its other types, and has haste. III — Sacrifice all Reflections you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("TheApprenticesFolly.TheApprenticesFollyPredicate", no_token_named_so)];

/// a nameless permanent, or none of the player's tokens has its name
fn no_token_named_so(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    let name = game.obj(id).name().to_string();
    if name.is_empty() {
        return true;
    }
    !game.battlefield.iter().any(|o| {
        let p = game.obj(*o);
        p.token && p.controller == you && !p.phased_out && p.name() == name
    })
}
