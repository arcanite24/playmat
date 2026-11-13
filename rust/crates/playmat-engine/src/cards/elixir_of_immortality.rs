//! Elixir of Immortality: "{2}, {T}: You gain 5 life. Shuffle Elixir of Immortality and your graveyard into their
//! owner's library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ElixirOfImmortality.ElixerOfImmortalityEffect", apply)];

/// You gain 5 life; the Elixir (still on the battlefield) and then each card of your graveyard go on top of your
/// library one by one; then you shuffle
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    gain_life(game, you, 5);
    if game.obj(cx.source_id()).zone == Zone::Battlefield {
        to_library(game, cx.source_id(), true);
    }
    for c in game.player(you).graveyard.clone() {
        to_library(game, c, true);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
