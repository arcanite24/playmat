//! Ephara, God of the Polis: "At the beginning of each upkeep, if you had another creature enter the battlefield
//! under your control last turn, draw a card."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("EpharaGodOfThePolis.EpharaGodOfThePolisCondition", another_entered_last_turn)];

/// `PermanentsEnteredBattlefieldWatcher.anotherCreatureEnteredBattlefieldUnderPlayersControlLastTurn`: a permanent
/// that entered under Ephara's controller last turn, a creature as it entered, other than Ephara (by object, as
/// Java compares the permanents' ids)
fn another_entered_last_turn(game: &Game, _you: Seat, source: ObjId) -> bool {
    let controller = game.obj(source).controller;
    game.watch.entered_last_turn.iter().any(|(seat, creature, id)| *seat == controller && *creature && *id != source)
}
