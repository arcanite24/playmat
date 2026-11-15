//! Runed Crown: "When Runed Crown enters, you may search your library, hand, and/or graveyard for a Rune card and
//! put it onto the battlefield attached to Runed Crown. If you search your library this way, shuffle. Equipped
//! creature gets +1/+1. Equip {2}"

use super::*;
use crate::card::{Filter, Pred};

pub const HANDLERS: &[Handler] = &[Handler::effect("RunedCrown.RunedCrownEffect", apply)];

/// "Search your graveyard for a Rune card?", then (none yet) "Search your hand for a Rune card?", then (none yet)
/// the library searched and shuffled; the Rune found enters attached to the Crown if it's still there (else it just
/// enters)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let filter = Filter::of(vec![Pred::Subtype { s: "Rune".into() }]);
    let matches = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        cards.iter().copied().filter(|c| game.filter_matches(&filter, you, Some(src), *c)).collect()
    };
    let mut card = None;
    if choose_use(game, d, you, "Search your graveyard for a Rune card?")? {
        let cards = game.player(you).graveyard.clone();
        let possible = matches(game, &cards);
        card = super::boonweaver_giant::choose_one(game, d, you, &cards, &possible)?;
    }
    if card.is_none() && choose_use(game, d, you, "Search your hand for a Rune card?")? {
        let cards = game.player(you).hand.clone();
        let possible = matches(game, &cards);
        card = super::boonweaver_giant::choose_one(game, d, you, &cards, &possible)?;
    }
    if card.is_none() {
        card = search_library(game, d, you, &filter, 1, 1, src)?.first().copied();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    let Some(card) = card else { return Some(()) };
    if cx.source_on_battlefield(game) {
        game.attach_entering = Some((card, src, game.obj(src).zcc));
    }
    put_onto_battlefield(game, d, card, you);
    game.attach_entering = None;
    if d.stopped() {
        return None;
    }
    Some(())
}
