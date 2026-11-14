//! Last Light of Durin's Day: "Whenever a Mountain you control enters, put a quest counter on Last Light of Durin's
//! Day. Then if it has six or more quest counters on it, sacrifice it. When you do, you may search your hand and/or
//! library for a Dragon card and put it onto the battlefield. If you search your library this way, shuffle.
//! Mountaincycling {2}"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LastLightOfDurinsDay.LastLightOfDurinsDayEffect", apply)];

/// The source still on the battlefield with six quest counters, sacrificed; then the hand is offered (a Dragon
/// chosen from it, not asked with an empty hand), then, with none, the library (searched, then shuffled); the card
/// found onto the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    if !cx.source_on_battlefield(game) || game.obj(src).counter("quest") < 6 {
        return Some(());
    }
    sacrifice(game, src);
    if game.obj(src).zone == Zone::Battlefield {
        return Some(());
    }
    let dragon = crate::card::Filter(vec![Pred::Subtype { s: "Dragon".into() }]);
    let mut card = None;
    if choose_use(game, d, you, "Search your hand for a Dragon card?")? {
        let hand = game.player(you).hand.clone();
        if !hand.is_empty() {
            let possible: Vec<ObjId> =
                hand.iter().copied().filter(|c| game.filter_matches(&dragon, you, Some(src), *c)).collect();
            let rule = item_rule(game, cx.item);
            card = choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?.first().copied();
        }
    }
    if card.is_none() && choose_use(game, d, you, "Search your library for a Dragon card?")? {
        card = search_library(game, d, you, &dragon, 1, 1, src)?.first().copied();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    if let Some(c) = card {
        if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    Some(())
}
