//! `SearchLibraryPutOntoBattlefieldTappedRestInHandEffect` (Cultivate, Peregrination, Troop of Ponies…): "search
//! your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the
//! other into your hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SearchLibraryPutOntoBattlefieldTappedRestInHandEffect", apply).args(&["library_target", "text"])];

/// The search; the found cards revealed; with two or more, the ones for the battlefield chosen from them
/// (`TargetCardInLibrary(n, …)` over the revealed cards, `numToBattlefield` of them, 1 by default) and the rest to
/// the hand; with one, it goes onto the battlefield tapped (a random pick of one draws nothing). Then the shuffle.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let bounds: Vec<usize> =
        cx.args.first().map(|a| a.split(' ').filter_map(|n| n.parse().ok()).collect()).unwrap_or_default();
    let (min, max) = match bounds[..] {
        [lo, hi] => (lo, hi),
        _ => (1, 1),
    };
    let to_battlefield = cx.arg_int(1).unwrap_or(1).max(0) as usize;
    let filter = cx.filters.last().cloned().unwrap_or_default();
    let found = search_library(game, d, you, &filter, min, max, cx.source_id())?;
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        if found.len() >= 2 {
            let n = to_battlefield.min(found.len());
            let rule = item_rule(game, cx.item);
            let chosen = choose_cards(game, d, you, &found, &found, n, n, &rule)?;
            if !chosen.is_empty() && !put_cards(game, d, you, &chosen, crate::card::PutCards::BattlefieldTapped) {
                return None;
            }
            let rest: Vec<ObjId> =
                found.iter().copied().filter(|c| !chosen.contains(c) && game.obj(*c).zone == Zone::Library).collect();
            put_cards(game, d, you, &rest, crate::card::PutCards::Hand);
        } else if !put_cards(game, d, you, &found, crate::card::PutCards::BattlefieldTapped) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
