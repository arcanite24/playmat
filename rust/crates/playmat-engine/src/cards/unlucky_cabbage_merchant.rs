//! Unlucky Cabbage Merchant: "When this creature enters, create a Food token. Whenever you sacrifice a Food, you may
//! search your library for a basic land card and put it onto the battlefield tapped. If you search your library this
//! way, put this creature on the bottom of its owner's library, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnluckyCabbageMerchant.UnluckyCabbageMerchantEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// Asked first; then the search (it always counts as searched), the land onto the battlefield tapped, the source
/// (still the same permanent) on the bottom of its owner's library, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Search your library for a basic land?")? {
        return Some(());
    }
    let found = search_library(game, d, you, &cx.filters[0], 1, 1, cx.source_id())?;
    if !found.is_empty() && !put_cards(game, d, you, &found[..1], crate::card::PutCards::BattlefieldTapped) {
        return None;
    }
    if cx.source_on_battlefield(game) && !put_on_library(game, d, you, &[cx.source_id()], false, true) {
        return None;
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
