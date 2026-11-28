//! Carnivorous Cultivator's Enroot: "Search your library for a land card, put it into your graveyard, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("CarnivorousCultivator.EnrootEffect", apply).filters(&["StaticFilters.FILTER_CARD_LAND"])];

/// `searchLibrary(TargetCardInLibrary(land card))`: the card found into the graveyard, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first().cloned().unwrap_or_default();
    let found = search_library(game, d, you, &filter, 0, 1, cx.source_id())?;
    for c in found {
        game.move_to(c, Zone::Graveyard);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
