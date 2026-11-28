//! Visions of Phyrexia: "At the beginning of your end step, if you didn't play a card from exile this turn, create a
//! tapped Powerstone token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("VisionsOfPhyrexia.VisionsOfPhyrexiaCondition", didnt_play)];

/// its watcher's set: who played a land from exile (`LAND_PLAYED` from exile) or cast a spell from exile
fn didnt_play(game: &Game, you: Seat, _source: ObjId) -> bool {
    !game.watch.played_from_exile.contains(&you)
}
