//! Secret Salvage: "Exile target nonland card from your graveyard. Search your library for any number of cards with
//! the same name as that card, reveal them, put them into your hand, then shuffle."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SecretSalvage.SecretSalvageEffect", apply)];

/// The card exiled, then `SearchLibraryPutInHandEffect(TargetCardInLibrary(0, MAX, named it), reveal)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Graveyard {
        return Some(());
    }
    let name = game.obj(c).name().to_string();
    if !put_cards(game, d, you, &[c], crate::card::PutCards::Exiled) {
        return None;
    }
    let filter = crate::card::Filter(vec![Pred::Name { n: name }]);
    let found = search_library(game, d, you, &filter, 0, usize::MAX, cx.source_id())?;
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        put_cards(game, d, you, &found, crate::card::PutCards::Hand);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
