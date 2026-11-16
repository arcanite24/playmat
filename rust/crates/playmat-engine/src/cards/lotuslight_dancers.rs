//! Lotuslight Dancers: "Lifelink. When this creature enters, search your library for a black card, a green card, and
//! a blue card. Put those cards into your graveyard, then shuffle."

use super::*;
use crate::card::Color;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LotuslightDancers.LotuslightDancersEffect", apply),
    Handler::predicate("LotuslightDancers.LotuslightDancersTarget", fits_roles),
];

/// `LotuslightDancersTarget` (0 to 3 cards), the ones found still in the library into the graveyard (in the order
/// chosen), then the shuffle
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let filter: Filter =
        serde_json::from_value(serde_json::json!([{"p": "card", "handler": "LotuslightDancers.LotuslightDancersTarget"}]))
            .ok()?;
    let found = search_library(game, d, you, &filter, 0, 3, cx.source_id())?;
    for c in found {
        if game.obj(c).zone == Zone::Library {
            game.move_to(c, Zone::Graveyard);
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

/// blue, black or green, and with the cards chosen so far (`game.searching`) each can still take a color of its own
/// (`ColorAssignment("U", "B", "G").hasSharedRoles` is false)
fn fits_roles(game: &Game, _: Seat, _: Option<ObjId>, id: ObjId) -> bool {
    let roles = |c: ObjId| -> Vec<Color> {
        game.colors(c).into_iter().filter(|k| matches!(k, Color::U | Color::B | Color::G)).collect()
    };
    let mut cards: Vec<Vec<Color>> = Vec::new();
    if let Some((_, chosen)) = &game.searching {
        cards.extend(chosen.iter().filter(|c| **c != id).map(|c| roles(*c)));
    }
    cards.push(roles(id));
    assignable(&cards, &mut Vec::new())
}

/// each card gets a distinct one of its colors
fn assignable(cards: &[Vec<Color>], used: &mut Vec<Color>) -> bool {
    let Some((first, rest)) = cards.split_first() else { return true };
    for k in first {
        if used.contains(k) {
            continue;
        }
        used.push(*k);
        let ok = assignable(rest, used);
        used.pop();
        if ok {
            return true;
        }
    }
    false
}
