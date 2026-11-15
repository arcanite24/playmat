//! Evolving Door: "{1}, {T}, Sacrifice a creature: Count the colors of the sacrificed creature, then search your
//! library for a creature card that's exactly that many colors plus one. Exile that card, then shuffle. You may cast
//! the exiled card. Activate only as a sorcery."

use super::*;

/// `EvolvingDoorPredicate(count)` for each count of `filterMap` (1 to 6)
const COLORS: [&str; 6] = [
    "EvolvingDoor.EvolvingDoorPredicate.1",
    "EvolvingDoor.EvolvingDoorPredicate.2",
    "EvolvingDoor.EvolvingDoorPredicate.3",
    "EvolvingDoor.EvolvingDoorPredicate.4",
    "EvolvingDoor.EvolvingDoorPredicate.5",
    "EvolvingDoor.EvolvingDoorPredicate.6",
];

pub const HANDLERS: &[Handler] = &[
    Handler::effect("EvolvingDoor.EvolvingDoorEffect", apply),
    Handler::predicate(COLORS[0], |g, _, _, id| colors(g, id) == 1),
    Handler::predicate(COLORS[1], |g, _, _, id| colors(g, id) == 2),
    Handler::predicate(COLORS[2], |g, _, _, id| colors(g, id) == 3),
    Handler::predicate(COLORS[3], |g, _, _, id| colors(g, id) == 4),
    Handler::predicate(COLORS[4], |g, _, _, id| colors(g, id) == 5),
    Handler::predicate(COLORS[5], |g, _, _, id| colors(g, id) == 6),
];

/// how many different colors a card is (`getColor(game).getColorCount()`)
fn colors(game: &Game, id: ObjId) -> usize {
    let mut seen = game.obj(id).def.colors.clone();
    seen.sort();
    seen.dedup();
    seen.len()
}

/// The sacrificed creature's colors (its card's: the engine keeps no last known colors) plus one; a creature card of
/// exactly that many colors searched for (`TargetCardInLibrary`, one), exiled, the library shuffled, then "Cast …?"
/// and the cast, its costs paid
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(&sacrificed) = cx.item.sacrificed.first() else { return Some(()) };
    let count = colors(game, sacrificed) + 1;
    let Some(name) = COLORS.get(count - 1) else { return Some(()) };
    let filter: crate::card::Filter =
        serde_json::from_value(serde_json::json!([{"p": "type", "t": "CREATURE"}, {"p": "card", "handler": name}]))
            .ok()?;
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    let card = found.first().copied();
    if let Some(c) = card {
        game.move_to(c, Zone::Exile);
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    let Some(card) = card else { return Some(()) };
    let message = format!("Cast {}?", game.obj(card).name());
    if !choose_use(game, d, you, &message)? || game.obj(card).zone != Zone::Exile {
        return Some(());
    }
    cast_allowed(game, d, you, card);
    (!d.stopped()).then_some(())
}
