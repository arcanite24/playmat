//! Rise of the Dread Marn: "Create X 2/2 black Zombie Berserker creature tokens, where X is the number of nontoken
//! creatures that died this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::value("RiseOfTheDreadMarn.RiseOfTheDreadMarnValue", died)];

/// `RiseOfTheDreadMarnWatcher`: nontoken creatures put into a graveyard from the battlefield this turn
fn died(game: &Game, _you: Seat, _source: ObjId, _x: i32) -> i32 {
    game.watch.to_graveyard.iter().filter(|(c, _, from_battlefield)| *from_battlefield && game.obj(*c).is(CardType::Creature)).count()
        as i32
}
