//! Kyoshi Island Plaza: "When Kyoshi Island Plaza enters, search your library for up to X basic land cards, where X
//! is the number of Shrines you control. Put those cards onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KyoshiIslandPlaza.KyoshiIslandPlazaEffect", apply).effects(&[
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(0, 1, StaticFilters.FILTER_CARD_BASIC_LANDS), true)",
])];

/// the Shrines counted as it resolves, as the search's most
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game.battlefield.iter().filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Shrine")).count();
    let mut e = cx.effects[0].clone();
    if let EffectDef::SearchLibraryEffect { max, min, .. } = &mut e {
        *max = x;
        *min = 0;
    }
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
