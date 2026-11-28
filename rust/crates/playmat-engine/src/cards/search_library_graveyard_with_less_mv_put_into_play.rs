//! `SearchLibraryGraveyardWithLessMVPutIntoPlay` (Finale of Devastation): "Search your library and/or graveyard for
//! a creature card with mana value X or less and put it onto the battlefield. If you search your library this way,
//! shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SearchLibraryGraveyardWithLessMVPutIntoPlay", apply).args(&["filter"])];

/// Asked "search your library?" first (then the search and the shuffle); with nothing found there, "search your
/// graveyard?" (`TargetCard(0, 1)` over the graveyard, asked when it isn't empty). The card found is revealed and put
/// onto the battlefield.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut filter = cx.filters.last().cloned().unwrap_or_default();
    filter.0.push(Pred::Mv { cmp: Cmp::FewerThan, n: cx.item.x + 1 });
    let mut found = None;
    let mut from = Zone::Library;
    if choose_use(game, d, you, "Search your library?")? {
        let cards = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
        found = cards.first().copied();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    if found.is_none() && choose_use(game, d, you, "Search your graveyard?")? {
        let grave = game.player(you).graveyard.clone();
        let possible: Vec<ObjId> =
            grave.iter().copied().filter(|c| game.filter_matches(&filter, you, Some(cx.source_id()), *c)).collect();
        if !grave.is_empty() {
            let rule = item_rule(game, cx.item);
            found = choose_cards(game, d, you, &grave, &possible, 0, 1, &rule)?.first().copied();
            from = Zone::Graveyard;
        }
    }
    if let Some(c) = found {
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards: vec![(c, from)] });
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    Some(())
}
