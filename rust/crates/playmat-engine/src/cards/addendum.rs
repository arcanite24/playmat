//! `AddendumCondition` (Arrester's Admonition…): "if you cast this spell during your main phase".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("AddendumCondition", addendum)];

/// Your turn, a main phase, and the source a spell on the stack
fn addendum(game: &Game, you: Seat, source: ObjId) -> bool {
    game.active == Some(you)
        && game.step.is_some_and(|s| s.is_main())
        && game.obj(source).zone == Zone::Stack
}
