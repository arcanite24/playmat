//! Lunar Convocation: "At the beginning of your end step, if you gained life this turn, each opponent loses 1 life.
//! At the beginning of your end step, if you gained and lost life this turn, create a 1/2 black Bat creature token
//! with flying." (`PlayerGainedLifeWatcher`, `PlayerLostLifeWatcher`)

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::condition("LunarConvocation.LunarConvocationCondition.GAINED", gained),
    Handler::condition("LunarConvocation.LunarConvocationCondition.GAINED_AND_LOST", gained_and_lost),
];

fn gained(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).life_gained > 0
}

fn gained_and_lost(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).life_gained > 0 && game.player(you).life_lost > 0
}
