//! Elixir: "Shuffle all nonland cards from your graveyard into your library. You gain life equal to the number of
//! cards shuffled into your library this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Elixir.ElixirEffect", apply)];

/// `shuffleCardsToLibrary` (nothing to do with no cards), then the life for those now in the library
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> =
        game.player(you).graveyard.iter().copied().filter(|c| !game.has_type(*c, CardType::Land)).collect();
    if cards.is_empty() {
        return Some(());
    }
    for c in &cards {
        to_library(game, *c, true);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    let n = cards.iter().filter(|c| game.obj(**c).zone == Zone::Library).count() as i32;
    if n > 0 {
        gain_life(game, you, n);
    }
    Some(())
}
