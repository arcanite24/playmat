//! Firemind's Foresight: "Search your library for an instant card with mana value 3, reveal it, and put it into your
//! hand. Then repeat this process for instant cards with mana values 2 and 1. Then shuffle."

use super::*;
use crate::card::{Cmp, Filter, PutCards, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("FiremindsForesight.FiremindsForesightSearchEffect", apply)];

/// Three searches (mana value 3, then 2, then 1), each card found revealed and put into the hand, then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for mv in (1..=3).rev() {
        let filter = Filter::of(vec![Pred::Type { t: CardType::Instant }, Pred::Mv { cmp: Cmp::EqualTo, n: mv }]);
        let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
        let Some(&card) = found.first() else { continue };
        let zcc = game.obj(cx.source_id()).zcc;
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        if !put_cards(game, d, you, &[card], PutCards::Hand) {
            return None;
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
