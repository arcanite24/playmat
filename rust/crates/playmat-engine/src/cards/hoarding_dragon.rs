//! Hoarding Dragon: "Flying. When Hoarding Dragon enters the battlefield, you may search your library for an
//! artifact card, exile it, then shuffle. When Hoarding Dragon dies, you may put the exiled card into its owner's
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HoardingDragon.HoardingDragonEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_ARTIFACT"])];

/// `searchLibrary(TargetCardInLibrary(artifact card))`: the card found goes into the source's exile zone
/// (`ExileTargetForSourceEffect`), then the library is shuffled, found or not
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 0, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile {
            let (src, zcc) = cx.source;
            game.exiled_for_source.push((src, zcc, card));
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
