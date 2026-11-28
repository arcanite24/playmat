//! Tiller of Flesh: "Whenever you cast a spell that targets one or more permanents, incubate 2."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("TillerOfFlesh.TillerOfFleshPredicate", targets_a_permanent)];

/// The spell's chosen targets include a permanent on the battlefield
fn targets_a_permanent(game: &Game, _you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.stack.iter().rev().find(|i| i.card == id && i.ability.is_none() && i.delayed.is_none()).is_some_and(|i| {
        i.targets.iter().flatten().any(|t| matches!(t, Target::Obj(o) if game.obj(*o).zone == Zone::Battlefield))
    })
}
