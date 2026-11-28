//! Field of Ruin: "{2}, {T}, Sacrifice Field of Ruin: Destroy target nonbasic land an opponent controls. Each player
//! searches their library for a basic land card, puts it onto the battlefield, then shuffles."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FieldOfRuin.FieldOfRuinEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_BASIC_LAND"])];

/// Each player in turn order from you searches for up to one basic land (`TargetCardInLibrary(0, 1, …)`), puts it
/// onto the battlefield and shuffles, before the next player searches
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..game.seats() {
        let p = (you + i) % game.seats();
        if game.player(p).lost {
            continue;
        }
        let found = search_library(game, d, p, &cx.filters[0], 0, 1, cx.source_id())?;
        if !found.is_empty() && !put_cards(game, d, p, &found, crate::card::PutCards::Battlefield) {
            return None;
        }
        let mut library = game.player(p).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(p).library = library;
    }
    Some(())
}
