//! `AddendumCondition` (Arrester's Admonition…): "if you cast this spell during your main phase".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("AddendumCondition", addendum)];

/// Your turn, a main phase, and the source a spell on the stack, or a permanent that was cast
/// (`CastFromEverywhereSourceCondition`: Sentinel's Mark's enters trigger)
fn addendum(game: &Game, you: Seat, source: ObjId) -> bool {
    let o = game.obj(source);
    game.active == Some(you)
        && game.step.is_some_and(|s| s.is_main())
        && (o.zone == Zone::Stack || o.cast || (o.zone != Zone::Battlefield && o.cast_lki))
}
