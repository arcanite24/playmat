//! The Three Seasons: "I — Mill three cards. II — Return up to two target snow permanent cards from your graveyard
//! to your hand. III — Choose three cards in each graveyard. Their owners shuffle those cards into their libraries."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheThreeSeasons.TheThreeSeasonsEffect", apply)];

/// For each player in range from you, with cards in their graveyard: you choose `min(3, its size)` of them
/// (`TargetCardInGraveyard(n)`, not targeted); then, in that order, each owner's `shuffleCardsToLibrary`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let seats = game.seats();
    let rule = item_rule(game, cx.item);
    let mut chosen: Vec<(Seat, Vec<ObjId>)> = Vec::new();
    for i in 0..seats {
        let p = (you + i) % seats;
        if game.player(p).lost {
            continue;
        }
        let graveyard = game.player(p).graveyard.clone();
        let n = graveyard.len().min(3);
        if n < 1 {
            continue;
        }
        let cards = choose_cards(game, d, you, &graveyard, &graveyard, n, n, &rule)?;
        chosen.push((p, cards));
    }
    for (p, cards) in chosen {
        let cards: Vec<ObjId> = cards.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
        if cards.is_empty() {
            continue;
        }
        for c in cards {
            to_library(game, c, true);
        }
        shuffle_library(game, p);
    }
    Some(())
}

fn shuffle_library(game: &mut Game, p: Seat) {
    let mut library = game.player(p).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(p).library = library;
}
