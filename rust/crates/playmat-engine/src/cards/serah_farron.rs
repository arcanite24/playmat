//! Serah Farron: "The first legendary creature spell you cast each turn costs {2} less to cast." (`SerahFarronPredicate`
//! on the reduction's filter, with `SerahFarronWatcher`: the players who cast a legendary creature spell this turn)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("SerahFarron.SerahFarronPredicate", first)];

/// `!SerahFarronWatcher.checkPlayer(playerId)`: you haven't cast a legendary creature spell this turn
fn first(game: &Game, you: Seat, _: Option<ObjId>, _: ObjId) -> bool {
    !game.watch.spells.iter().any(|s| {
        s.caster == you && s.creature && game.obj(s.card).def.supertypes.iter().any(|t| t == "LEGENDARY")
    })
}
