//! Faunsbane Troll: "When this creature enters, create a Monster Role token attached to it. {1}, Sacrifice an Aura
//! attached to this creature: This creature fights target creature you don't control. If that creature would die
//! this turn, exile it instead. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("FaunsbaneTroll.FaunsbaneTrollPredicate", attached_to_source)];

/// it's attached to the ability's source (`getSourceId().equals(getAttachedTo())`)
fn attached_to_source(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    source.is_some_and(|s| game.obj(id).attached_to.is_some_and(|(t, _)| t == s))
}
