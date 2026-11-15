//! Old-Growth Dryads: "When Old-Growth Dryads enters, each opponent may search their library for a basic land card,
//! put it onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OldGrowthDryads.OldGrowthDryadsEffect", apply).effects(&[
    "new SearchLibraryPutInPlayEffect(new TargetCardInLibrary(StaticFilters.FILTER_CARD_BASIC_LAND), true)",
])];

/// each opponent in turn order is asked, then searches as the controller of the search; those who found a card
/// shuffle afterwards (`playersThatSearched`: one who found nothing doesn't)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(EffectDef::SearchLibraryEffect { filter, to, .. }) = cx.effects.first() else { return Some(()) };
    let you = cx.controller();
    let mut searched = Vec::new();
    for i in 1..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let message = "Search your library for a basic land card and put it onto the battlefield tapped?";
        if choose_use(game, d, p, message)? {
            let found = search_library(game, d, p, filter, 1, 1, cx.source_id())?;
            if let Some(card) = found.first() {
                if !put_cards(game, d, p, &[*card], *to) {
                    return None;
                }
                searched.push(p);
            }
        }
    }
    for p in searched {
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
