//! Oriq Loremage: "{T}: Search your library for a card, put it into your graveyard, then shuffle. If it's an instant
//! or sorcery card, put a +1/+1 counter on Oriq Loremage."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("OriqLoremage.OriqLoremageEffect", apply)];

/// The search for any one card, that card into the graveyard, the shuffle (found or not); then an instant or sorcery
/// card puts a +1/+1 counter on the source while it's still on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let found = search_library(game, d, you, &crate::card::Filter::default(), 1, 1, cx.source_id())?;
    let shuffle = |game: &mut Game| {
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    };
    let Some(&card) = found.first() else {
        shuffle(game);
        return Some(());
    };
    game.move_to(card, Zone::Graveyard);
    shuffle(game);
    let def = &game.obj(card).def;
    if !(def.is(CardType::Instant) || def.is(CardType::Sorcery)) || !cx.source_on_battlefield(game) {
        return Some(());
    }
    put_counters(game, cx.source_id(), "+1/+1", 1, you);
    Some(())
}
