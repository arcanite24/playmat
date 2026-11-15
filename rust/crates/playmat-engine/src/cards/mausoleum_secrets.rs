//! Mausoleum Secrets: "Undergrowth — Search your library for a black card with mana value less than or equal to the
//! number of creature cards in your graveyard, reveal it, put it into your hand, then shuffle."

use super::*;
use crate::card::{Cmp, Color, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("MausoleumSecrets.MausoleumSecretsEffect", apply)];

/// the creature cards in your graveyard, then `SearchLibraryPutInHandEffect(TargetCardInLibrary(filter), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.player(you).graveyard.iter().filter(|c| game.obj(**c).is(CardType::Creature)).count() as i32;
    let filter = Filter(vec![Pred::Color { c: Color::B }, Pred::Mv { cmp: Cmp::FewerThan, n: n + 1 }]);
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        game.revealed.push(crate::state::Revealed {
            source: cx.source_id(),
            zcc: cx.source.1,
            paren: false,
            cards: vec![(card, Zone::Library)],
        });
        game.move_to(card, Zone::Hand);
    }
    cx.run(game, d, &[EffectDef::ShuffleLibrarySourceEffect]);
    Some(())
}
