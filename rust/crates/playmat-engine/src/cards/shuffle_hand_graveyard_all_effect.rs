//! `ShuffleHandGraveyardAllEffect` (Emergency Powers): "each player shuffles their hand and graveyard into their
//! library".

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShuffleHandGraveyardAllEffect", apply)];

/// Each player from you in turn order: the hand, then the graveyard, each card put on top in turn
/// (`moveCards(…, LIBRARY)`), then the library shuffled
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 0..game.seats() {
        let p = (you + k) % game.seats();
        if game.player(p).lost {
            continue;
        }
        for c in game.player(p).hand.clone() {
            to_library(game, c, true);
        }
        for c in game.player(p).graveyard.clone() {
            to_library(game, c, true);
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
