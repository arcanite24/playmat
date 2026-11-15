//! Tome of Legends: "This artifact enters with a page counter on it. Whenever your commander enters or attacks, put a
//! page counter on this artifact. {1}, {T}, Remove a page counter from this artifact: Draw a card."
//!
//! `CommanderPredicate`: a duel has no commanders, so nothing is your commander, and the "enters or attacks" trigger
//! (`EntersBattlefieldOrAttacksAllTriggeredAbility` of that filter) never fires: its enters half stands for it here.

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("CommanderPredicate", no_commander),
    Handler::trigger(
        "TomeOfLegends.EntersBattlefieldOrAttacksAllTriggeredAbility",
        "new EntersBattlefieldAllTriggeredAbility($0, $1)",
        always,
    ),
];

/// `CommanderPredicate`: a commander of a player in the game (there are none in a duel)
fn no_commander(_: &Game, _: Seat, _: Option<ObjId>, _: ObjId) -> bool {
    false
}

fn always(_: &Game, _: &StackItem) -> bool {
    true
}
