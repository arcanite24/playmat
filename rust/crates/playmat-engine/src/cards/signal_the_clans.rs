//! Signal the Clans: "Search your library for three creature cards and reveal them. If you reveal three cards with
//! different names, choose one of them at random and put that card into your hand. Shuffle the rest into your
//! library."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("SignalTheClans.SignalTheClansEffect", apply)
    .filters(&["StaticFilters.FILTER_CARD_CREATURE"])];

/// `TargetCardInLibrary(3, creature cards)`: the found cards are revealed (in the order chosen); three with different
/// names, one of them at random (`randomFromCollection`) goes to the hand; the rest are shuffled into the library
/// (`shuffleCardsToLibrary`), or with none left, a shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = cx.filters.first()?;
    let mut found = search_library(game, d, you, filter, 3, 3, cx.source_id())?;
    if !found.is_empty() {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc: cx.source.1,
            paren: false,
            cards: found.iter().map(|c| (*c, game.obj(*c).zone)).collect(),
        });
        let mut names: Vec<&str> = found.iter().map(|c| game.obj(*c).name()).collect();
        names.sort_unstable();
        names.dedup();
        if names.len() == 3 {
            let at = game.rng.next_int_bound(found.len() as i32) as usize;
            let card = found.remove(at);
            game.move_to(card, Zone::Hand);
        }
    }
    if !found.is_empty() {
        if !put_cards(game, d, you, &found, PutCards::Shuffle) {
            return None;
        }
    } else {
        cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    }
    Some(())
}
