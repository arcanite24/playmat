//! Teysa, Opulent Oligarch: "Deathtouch. At the beginning of your end step, investigate for each opponent who lost
//! life this turn. Whenever a Clue you control is put into a graveyard from the battlefield, create a 1/1 white and
//! black Spirit creature token with flying. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::value("TeysaOpulentOligarch.TeysaOpulentOligarchDynamicValue", opponents_lost_life)];

/// the opponents in range who lost life this turn (`PlayerLostLifeWatcher`)
fn opponents_lost_life(game: &Game, you: Seat, _: ObjId, _: i32) -> i32 {
    (0..game.seats()).filter(|p| *p != you && !game.player(*p).lost && game.player(*p).life_lost > 0).count() as i32
}
