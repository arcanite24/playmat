//! Jarad's Orders: "Search your library for up to two creature cards and reveal them. Put one into your hand and the
//! other into your graveyard. Then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JaradsOrders.JaradsOrdersEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURES"])];

/// The search; the found cards revealed; with two, the one for the hand chosen and the other to the graveyard; with
/// one, it goes to the hand. Then the shuffle.
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &cx.filters[0], 0, 2, cx.source_id())?;
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        if found.len() == 2 {
            let rule = item_rule(game, cx.item);
            let chosen = choose_cards(game, d, you, &found, &found, 1, 1, &rule)?;
            put_cards(game, d, you, &chosen, crate::card::PutCards::Hand);
            let rest: Vec<ObjId> =
                found.iter().copied().filter(|c| !chosen.contains(c) && game.obj(*c).zone == Zone::Library).collect();
            put_cards(game, d, you, &rest[..rest.len().min(1)], crate::card::PutCards::Graveyard);
        } else {
            put_cards(game, d, you, &found, crate::card::PutCards::Hand);
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
