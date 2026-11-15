//! Cranial Archive: "{2}, Exile Cranial Archive: Target player shuffles their graveyard into their library. Draw a
//! card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("CranialArchive.CranialArchiveEffect", apply)];

/// Each graveyard card onto their library (on top), the shuffle, then you draw (even with the player gone)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if let Some(p) = cx.player(0) {
        for c in game.player(p).graveyard.clone() {
            to_library(game, c, true);
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    draw(game, you, 1);
    Some(())
}
