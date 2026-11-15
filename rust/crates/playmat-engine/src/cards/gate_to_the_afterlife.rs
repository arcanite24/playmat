//! Gate to the Afterlife: "Whenever a nontoken creature you control dies, you gain 1 life. Then you may draw a card.
//! If you do, discard a card. {2}, {T}, Sacrifice Gate to the Afterlife: Search your graveyard, hand, and/or library
//! for a card named God-Pharaoh's Gift and put it onto the battlefield. If you search your library this way, shuffle.
//! Activate only if there are six or more creature cards in your graveyard."

use super::*;
use crate::card::{Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("GateToTheAfterlife.GateToTheAfterlifeEffect", apply)];

const NAME: &str = "God-Pharaoh's Gift";

/// "Search your graveyard for God-Pharaoh's Gift?" (a required one-card choice), then (none yet) "Search your hand
/// …?" (up to one), then (none yet) "Search your library …?": searched and shuffled; the card found onto the
/// battlefield, and the library shuffled again after a search (Java shuffles it twice)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let filter = Filter::of(vec![Pred::Name { n: NAME.into() }]);
    let matches = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| game.filter_matches(&filter, you, Some(src), *c)).collect()
    };
    let shuffle = |game: &mut Game| {
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    };
    let mut card = None;
    if choose_use(game, d, you, &format!("Search your graveyard for {NAME}?"))? {
        let cards = game.player(you).graveyard.clone();
        let possible = matches(game, &cards);
        card = super::boonweaver_giant::choose_one(game, d, you, &cards, &possible)?;
    }
    if card.is_none() && choose_use(game, d, you, &format!("Search your hand for {NAME}?"))? {
        let cards = game.player(you).hand.clone();
        let possible = matches(game, &cards);
        if !cards.is_empty() {
            card = super::gilgamesh_master_at_arms::choose_any(game, d, you, &possible, 1)?.first().copied();
        }
    }
    let mut searched = false;
    if card.is_none() && choose_use(game, d, you, &format!("Search your library for {NAME}?"))? {
        searched = true;
        card = search_library(game, d, you, &filter, 1, 1, src)?.first().copied();
        shuffle(game);
    }
    if let Some(card) = card {
        if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    if searched {
        shuffle(game);
    }
    Some(())
}
