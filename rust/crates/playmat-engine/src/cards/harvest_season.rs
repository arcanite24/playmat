//! Harvest Season: "Search your library for up to X basic land cards, where X is the number of tapped creatures you
//! control, put those cards onto the battlefield tapped, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HarvestSeason.HarvestSeasonEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND", "StaticFilters.FILTER_CONTROLLED_CREATURE"])];

/// With no tapped creature there's no search, only the shuffle (Java searches only for X > 0)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let tapped = matching(game, &cx.filters[1], you, cx.source_id()).into_iter().filter(|o| game.obj(*o).tapped).count();
    if tapped > 0 {
        let found = search_library(game, d, you, &cx.filters[0], 0, tapped, cx.source_id())?;
        if !found.is_empty() && !put_cards(game, d, you, &found, crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
