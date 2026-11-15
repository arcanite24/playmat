//! Vivien on the Hunt: "+2: You may sacrifice a creature. If you do, search your library for a creature card with
//! mana value equal to 1 plus the sacrificed creature's mana value, put it onto the battlefield, then shuffle. +1:
//! Mill five cards, then put any number of creature cards milled this way into your hand. −1: Create a 4/4 green Rhino
//! Warrior creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("VivienOnTheHunt.VivienOnTheHuntSacrificeEffect", sacrifice_and_search)
        .filters(&["StaticFilters.FILTER_CONTROLLED_ANOTHER_CREATURE"]),
    Handler::effect("VivienOnTheHunt.VivienOnTheHuntMillEffect", mill),
];

/// `TargetSacrifice(0, 1, another creature you control)` chosen with `Player.choose`; once it's sacrificed, a search
/// for a creature card of its mana value plus 1 (as it last was), the card found onto the battlefield, the shuffle
fn sacrifice_and_search(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let possible: Vec<ObjId> = matching(game, &cx.filters[0], you, cx.source_id())
        .into_iter()
        .filter(|o| game.obj(*o).controller == you && *o != cx.source_id())
        .collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &possible, &possible, 0, 1, &rule)?;
    let Some(&o) = chosen.first() else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let mv = game.obj(o).def.mana_value() as i32 + 1;
    sacrifice(game, o);
    if game.obj(o).zone == Zone::Battlefield {
        return Some(());
    }
    let filter: crate::card::Filter = serde_json::from_value(serde_json::json!([
        {"p": "type", "t": "CREATURE"},
        {"p": "mv", "cmp": "EQUAL_TO", "n": mv}
    ]))
    .ok()?;
    let found = search_library(game, d, you, &filter, 1, 1, cx.source_id())?;
    if let Some(&card) = found.first() {
        if game.obj(card).zone == Zone::Library {
            put_onto_battlefield(game, d, card, you);
            if d.stopped() {
                return None;
            }
        }
    }
    let mut library = game.player(you).library.clone();
    game.rng.shuffle(&mut library);
    game.player_mut(you).library = library;
    Some(())
}

/// Five cards milled; with a creature card among them, `TargetCard(0, MAX, creature)` of the milled cards chosen
/// with `Player.choose`, those to your hand
fn mill(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    note(game, EventKind::Milled, Some(you), None, None, top.len() as i32);
    for c in &top {
        game.move_to(*c, Zone::Graveyard);
    }
    let milled: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Graveyard).collect();
    let creatures: Vec<ObjId> = milled.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    if creatures.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &milled, &creatures, 0, usize::MAX, &rule)?;
    for c in chosen {
        if game.obj(c).zone == Zone::Graveyard {
            game.move_to(c, Zone::Hand);
        }
    }
    Some(())
}
