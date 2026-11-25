//! 70,000 Light-Years from Home: "Exile target creature or Spacecraft. Its controller may search their library for a
//! basic land card, reveal it, put it into their hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SeventyThousandLightYearsFromHome.SeventyThousandLightYearsFromHomeEffect", apply).effects(&[
        "new SearchLibraryPutInHandEffect(new TargetCardInLibrary(StaticFilters.FILTER_CARD_BASIC_LAND), true)",
    ])];

/// The exiled permanent's controller as it last was (`getControllerOfFirstTargetOrLKI`): "Search your library for a
/// basic land card?"; no does nothing (no shuffle), yes is the search, reveal, hand and shuffle
/// (`SearchLibraryPutInHandEffect`'s, as that player)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    let p = game.obj(o).controller;
    if !choose_use(game, d, p, "Search your library for a basic land card?")? {
        return Some(());
    }
    cx.run_as(game, d, cx.effects, p);
    Some(())
}
