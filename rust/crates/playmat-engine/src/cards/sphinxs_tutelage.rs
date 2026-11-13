//! Sphinx's Tutelage: "Whenever you draw a card, target opponent mills two cards. If two nonland cards that share a
//! color were milled this way, repeat this process."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SphinxsTutelage.SphinxsTutelageEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
    let mill = [EffectDef::MillCardsControllerEffect { amount: crate::card::Amount::Fixed(2) }];
    // Java's loop guard: a draw once it runs past the library's size
    let limit = game.player(p).library.len() / 2 + 20;
    for _ in 0..=limit {
        let top: Vec<ObjId> = game.player(p).library.iter().take(2).copied().collect();
        cx.run_as(game, d, &mill, p);
        let cards: Vec<ObjId> = top
            .into_iter()
            .filter(|c| game.obj(*c).zone == Zone::Graveyard && !game.obj(*c).is(CardType::Land))
            .collect();
        if cards.len() < 2 {
            return Some(());
        }
        let (a, b) = (game.colors(cards[0]), game.colors(cards[1]));
        if !a.iter().any(|c| b.contains(c)) {
            return Some(());
        }
    }
    Some(())
}
