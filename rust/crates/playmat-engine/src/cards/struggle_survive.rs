//! Struggle // Survive: "Survive — Each player shuffles their graveyard into their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StruggleSurvive.SurviveEffect", apply)];

/// Each player in range from you, in turn order: their graveyard into their library and the shuffle (nothing, not
/// even a shuffle, for an empty one: `shuffleCardsToLibrary`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let cards = game.player(p).graveyard.clone();
        if cards.is_empty() {
            continue;
        }
        for c in cards {
            to_library(game, c, true);
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
