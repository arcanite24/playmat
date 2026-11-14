//! Serene Remembrance: "Shuffle Serene Remembrance and up to three target cards from a single graveyard into their
//! owners' libraries."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SereneRemembrance.SereneRemembranceEffect", apply)];

/// `shuffleCardsToLibrary` twice: this card (from the stack) into your library and your shuffle, then the targets
/// into their owner's library and that owner's shuffle
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let me = cx.source_id();
    if matches!(game.obj(me).zone, Zone::Stack | Zone::Graveyard | Zone::Hand | Zone::Exile) {
        to_library(game, me, true);
        shuffle_library(game, you);
    }
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    if let Some(&first) = cards.first() {
        let owner = game.obj(first).owner;
        for c in cards {
            to_library(game, c, true);
        }
        shuffle_library(game, owner);
    }
    Some(())
}

fn shuffle_library(game: &mut Game, p: Seat) {
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
}
