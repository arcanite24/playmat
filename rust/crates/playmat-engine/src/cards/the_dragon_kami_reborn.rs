//! The Dragon-Kami Reborn // Dragon-Kami's Egg: "I, II — You gain 2 life. Look at the top three cards of your
//! library. Exile one of them face down with a hatching counter on it, then put the rest on the bottom of your
//! library in any order." and "Whenever Dragon-Kami's Egg or a Dragon you control dies, you may cast a creature
//! spell from among cards you own in exile with hatching counters on them without paying its mana cost."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TheDragonKamiReborn.TheDragonKamiRebornEffect", hatch),
    Handler::effect("TheDragonKamiReborn.DragonKamisEggEffect", cast_hatched).filters(&["StaticFilters.FILTER_CARD_CREATURE"]),
];

/// `player.choose(outcome, cards, new TargetCardInLibrary(), …)` of the top three (required), moved to exile with a
/// hatchling counter (the engine keeps no face-down exile: Java only hides it); the rest still in the library to the
/// bottom in any order
fn hatch(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let top: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    if top.is_empty() {
        return Some(());
    }
    if let Some(&card) = choose_cards(game, d, you, &top, &top, 1, 1, &item_rule(game, cx.item))?.first() {
        game.move_to(card, Zone::Exile);
        if game.obj(card).zone == Zone::Exile {
            game.obj_mut(card).add_counters("hatchling", 1);
        }
    }
    let rest: Vec<ObjId> = top.into_iter().filter(|c| game.obj(*c).zone == Zone::Library).collect();
    if !put_on_library(game, d, you, &rest, false, true) {
        return None;
    }
    Some(())
}

/// `CardUtil.castSpellWithAttributesForFree` over the exiled cards you own with a hatchling counter, creature parts
fn cast_hatched(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> = game
        .exile
        .iter()
        .copied()
        .filter(|c| game.obj(*c).zone == Zone::Exile && game.obj(*c).owner == you && game.obj(*c).counter("hatchling") > 0)
        .collect();
    if cards.is_empty() {
        return Some(());
    }
    if !cast_one_for_free_from(game, d, you, cx.source_id(), &cx.filters[0], &cards) {
        return None;
    }
    Some(())
}
