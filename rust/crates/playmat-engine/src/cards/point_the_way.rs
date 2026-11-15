//! Point the Way: "{3}{G}, Sacrifice this enchantment: Search your library for up to X basic land cards, where X is
//! your speed. Put those cards onto the battlefield tapped, then shuffle."

use super::*;

/// the search for up to 0, 1, 2, 3 or 4 basic land cards (speed is never more than 4)
pub const HANDLERS: &[Handler] = &[Handler::effect("PointTheWay.PointTheWayEffect", apply).effects(&[
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(0,0,StaticFilters.FILTER_CARD_BASIC_LANDS),true)",
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(0,1,StaticFilters.FILTER_CARD_BASIC_LANDS),true)",
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(0,2,StaticFilters.FILTER_CARD_BASIC_LANDS),true)",
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(0,3,StaticFilters.FILTER_CARD_BASIC_LANDS),true)",
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(0,4,StaticFilters.FILTER_CARD_BASIC_LANDS),true)",
])];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let speed = (game.player(cx.controller()).speed as usize).min(4);
    cx.run(game, d, &cx.effects[speed..speed + 1]);
    Some(())
}
