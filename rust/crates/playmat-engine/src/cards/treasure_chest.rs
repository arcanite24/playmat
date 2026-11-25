//! Treasure Chest, its 20: "Search your library for a card. If it's an artifact card, you may put it onto the
//! battlefield. Otherwise, put that card into your hand. Then shuffle."

use super::*;
use crate::card::{Filter, PutCards};

pub const HANDLERS: &[Handler] = &[Handler::effect("TreasureChest.TreasureChestEffect", apply)];

fn shuffle(game: &mut Game, seat: Seat) {
    let mut library = game.player(seat).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(seat).library = library;
}

/// a `TargetCardInLibrary()` search; nothing found, a shuffle; an artifact card asks "Put it onto the battlefield or
/// your hand?" (battlefield: yes), anything else goes to the hand; then a shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &Filter(Vec::new()), 1, 1, cx.source_id())?;
    let Some(&card) = found.first() else {
        shuffle(game, you);
        return Some(());
    };
    let battlefield =
        game.has_type(card, CardType::Artifact) && choose_use(game, d, you, "Put it onto the battlefield or your hand?")?;
    let to = if battlefield { PutCards::Battlefield } else { PutCards::Hand };
    if !put_cards(game, d, you, &[card], to) {
        return None;
    }
    shuffle(game, you);
    Some(())
}
