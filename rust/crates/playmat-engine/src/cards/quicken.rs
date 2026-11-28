//! Quicken: "The next sorcery spell you cast this turn can be cast as though it had flash. Draw a card."
//!
//! `QuickenAsThoughEffect` (until end of turn) asks `QuickenWatcher`, which forgets every Quicken once its controller
//! (the player whose Quicken registered the game-wide watcher: Quicken's in a duel with one player's deck) casts a
//! sorcery spell. The card it lets be cast is checked as a card (`game.getCard(sourceId).isSorcery`), not as the face.

use super::*;
use crate::card::CardDef;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("Quicken.QuickenAsThoughEffect", AsThough::Flash(flash)).on_start(start)];

/// the sorcery spells `seat` has cast this turn (`SPELL_CAST` of a sorcery spell, as `QuickenWatcher` sees it)
fn sorceries_cast(game: &Game, seat: Seat) -> i32 {
    game.watch
        .spells
        .iter()
        .filter(|s| s.caster == seat && !s.creature && game.obj(s.card).def.is(CardType::Sorcery))
        .count() as i32
}

/// `init`: the watcher's list gets this Quicken; kept as the sorceries cast so far, so a later one ends it
fn start(game: &mut Game, _d: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let n = sorceries_cast(game, cx.controller);
    cx.set_left(game, n);
    Some(cx.targets.to_vec())
}

fn flash(game: &Game, cx: &StaticCx, seat: Seat, id: ObjId, _face: &CardDef) -> bool {
    seat == cx.controller
        && cx.left(game, 0) == sorceries_cast(game, cx.controller)
        && game.obj(id).def.is(CardType::Sorcery)
}
