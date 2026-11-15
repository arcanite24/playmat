//! Shadow Urchin: "Whenever this creature attacks, blight 1. Whenever a creature you control with one or more counters
//! on it dies, exile that many cards from the top of your library. Until your next end step, you may play those
//! cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("ShadowUrchin.ShadowUrchinValue", died_counters)];

/// `getValue("creatureDied")`'s counters of every kind as it last was on the battlefield (none: 0)
fn died_counters(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    let Some((died, _)) = game.resolving_item.as_ref().and_then(|i| i.died) else { return 0 };
    game.obj(died).lki_counters.iter().map(|(_, n)| *n).sum()
}
