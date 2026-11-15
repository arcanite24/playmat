//! `SearchLibraryGraveyardPutOntoBattlefieldEffect(filter)` (Elspeth, Undaunted Hero, Sanctum of All): "Search your
//! library and/or graveyard for a <filter> and put it onto the battlefield. If you search your library this way,
//! shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SearchLibraryGraveyardPutOntoBattlefieldEffect", apply)
    .args(&["filter", "one_of:false", "one_of:false"])];

/// Asked "search your library?" first (then `TargetCardInLibrary(0, 1)`); with nothing found there, "search your
/// graveyard?" (`TargetCardInYourGraveyard(0, 1)`, asked when the graveyard isn't empty). The card found goes onto
/// the battlefield, then the library is shuffled if it was searched.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let mut found = None;
    let mut shuffle = false;
    if choose_use(game, d, you, "Search your library?")? {
        let cards = search_library(game, d, you, &filter, 0, 1, cx.source_id())?;
        found = cards.first().copied();
        shuffle = true;
    }
    if found.is_none() && choose_use(game, d, you, "Search your graveyard?")? {
        let grave = game.player(you).graveyard.clone();
        let possible: Vec<ObjId> =
            grave.iter().copied().filter(|c| game.filter_matches(&filter, you, Some(cx.source_id()), *c)).collect();
        if !grave.is_empty() {
            let rule = item_rule(game, cx.item);
            found = choose_cards(game, d, you, &grave, &possible, 0, 1, &rule)?.first().copied();
        }
    }
    if let Some(c) = found {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    if shuffle {
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    Some(())
}
