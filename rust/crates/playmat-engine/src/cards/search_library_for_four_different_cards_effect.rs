//! `SearchLibraryForFourDifferentCardsEffect(filter, putCards, useTargetPointer)` (Elemental Teachings, Gifts
//! Ungiven, Realms Uncharted): "search your library for up to four [cards] with different names and reveal them.
//! [Target / An] opponent chooses two of those cards. Put the chosen cards into your graveyard and the rest [into your
//! hand / onto the battlefield tapped]. Then shuffle"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SearchLibraryForFourDifferentCardsEffect", apply).args(&[
    "filter",
    "one_of:PutCards.HAND|PutCards.BATTLEFIELD|PutCards.BATTLEFIELD_TAPPED",
    "one_of:true|false",
])];

/// `TargetCardWithDifferentNameInLibrary(0, 4, filter)`, revealed; none found: the shuffle. More than two: the
/// opponent (the target, or the one opponent) chooses two (`TargetCard(2, LIBRARY)`) for the graveyard, else both go
/// there; the rest are put as `putCards` says, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (Some(filter), [_, to, pointer]) = (cx.filters.first(), cx.args) else { return Some(()) };
    let to = to.trim().strip_prefix("PutCards.")?;
    let to = serde_json::from_value::<crate::card::PutCards>(serde_json::Value::String(to.into())).ok()?;
    let you = cx.controller();
    let mut filter = filter.clone();
    filter.0.push(crate::card::Pred::DifferentName);
    let found = search_library(game, d, you, &filter, 0, 4, cx.source_id())?;
    let found: Vec<ObjId> = found.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
    }
    if !found.is_empty() {
        let to_grave = if found.len() > 2 {
            let opponent = if pointer.trim() == "true" { cx.player(0) } else { Some(game.opponent(you)) };
            match opponent {
                Some(p) => choose_cards(game, d, p, &found, &found, 2, 2, &item_rule(game, cx.item))?,
                None => Vec::new(),
            }
        } else {
            found.clone()
        };
        for c in &to_grave {
            game.move_to(*c, Zone::Graveyard);
        }
        let rest: Vec<ObjId> = found.iter().copied().filter(|c| !to_grave.contains(c)).collect();
        if !rest.is_empty() && !put_cards(game, d, you, &rest, to) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
