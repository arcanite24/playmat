//! Poison the Cup: "Destroy target creature. If this spell was foretold, scry 2."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("ForetoldCondition", foretold)];

/// `ForetoldWatcher.checkForetold`: the source is the spell cast from the exile it was foretold into (Java's
/// reference is to that spell, so the permanent it resolves into isn't); a spell keeps `foretold_turn` until it leaves the stack
fn foretold(game: &Game, _: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    o.zone == Zone::Stack && o.cast_from == Some(Zone::Exile) && o.foretold_turn.is_some()
}
