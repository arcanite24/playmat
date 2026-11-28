//! `ShuffleHandGraveyardIntoLibraryEffect` (Midnight Clock): "shuffle your hand and graveyard into your library"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShuffleHandGraveyardIntoLibraryEffect", apply)];

/// `shuffleCardsToLibrary` of your hand and graveyard: nothing when both are empty, else each card into the library,
/// then the shuffle
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let player = game.player(you);
    let cards: Vec<ObjId> = player.hand.iter().chain(player.graveyard.iter()).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    for c in cards {
        to_library(game, c, true);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
