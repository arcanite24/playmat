//! Kogla and Yidaro's "{2}{R}{G}, Discard Kogla and Yidaro: Destroy up to one target artifact or enchantment.
//! Shuffle Kogla and Yidaro into your library from your graveyard, then draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KoglaAndYidaro.KoglaAndYidaroEffect", apply)];

/// The card discarded for the cost (`MageObjectReference(source, 1)`: the zone change after the activation's), still
/// in the graveyard, is shuffled into your library (`shuffleCardsToLibrary`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (o, zcc) = cx.source;
    if game.obj(o).zone != Zone::Graveyard || !(game.obj(o).zcc == zcc + 1 || game.obj(o).zcc == zcc) {
        return Some(());
    }
    let you = cx.controller();
    to_library(game, o, true);
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
