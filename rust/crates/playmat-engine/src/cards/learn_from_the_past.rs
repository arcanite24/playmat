//! Learn from the Past: "Target player shuffles their graveyard into their library. Draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LearnFromThePast.LearnFromThePastEffect", apply)];

/// Each card of the graveyard onto the library (`moveCardToLibraryWithInfo`, on top), then the shuffle
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    for c in game.player(p).graveyard.clone() {
        to_library(game, c, true);
    }
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
    Some(())
}
