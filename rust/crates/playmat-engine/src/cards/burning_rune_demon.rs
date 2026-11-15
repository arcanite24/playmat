//! Burning-Rune Demon: "Flying. When Burning-Rune Demon enters, you may search your library for exactly two cards not
//! named Burning-Rune Demon that have different names. If you do, reveal those cards. An opponent chooses one of
//! them. Put the chosen card into your hand and the other into your graveyard, then shuffle."

use super::*;
use crate::card::Pred;

pub const HANDLERS: &[Handler] = &[Handler::effect("BurningRuneDemon.BurningRuneDemonEffect", apply)];

/// `TargetCardWithDifferentNameInLibrary(2, 2, not named Burning-Rune Demon)`; found, revealed and the one opponent
/// chooses one (`TargetCard(LIBRARY)`) for your hand, the other to your graveyard; then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter = Filter::of(vec![
        Pred::Not { x: Box::new(Pred::Name { n: "Burning-Rune Demon".into() }) },
        Pred::DifferentName,
    ]);
    let found = search_library(game, d, you, &filter, 2, 2, cx.source_id())?;
    let found: Vec<ObjId> = found.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !found.is_empty() {
        let zcc = game.obj(cx.source_id()).zcc;
        let cards = found.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source: cx.source_id(), zcc, paren: false, cards });
        let opponent = game.opponent(you);
        let chosen = choose_cards(game, d, opponent, &found, &found, 1, 1, &item_rule(game, cx.item))?;
        if let Some(&c) = chosen.first() {
            game.move_to(c, Zone::Hand);
        }
        for c in found.iter().filter(|c| !chosen.contains(c)) {
            game.move_to(*c, Zone::Graveyard);
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}
