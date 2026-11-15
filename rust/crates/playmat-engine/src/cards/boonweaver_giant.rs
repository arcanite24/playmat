//! Boonweaver Giant: "When Boonweaver Giant enters the battlefield, you may search your graveyard, hand, and/or
//! library for an Aura card and put it onto the battlefield attached to Boonweaver Giant. If you search your library
//! this way, shuffle."

use super::*;
use crate::card::{Filter, Pred};

pub const HANDLERS: &[Handler] = &[
    Handler::effect("BoonweaverGiant.BoonweaverGiantEffect", apply),
    // the effect's own `AuraCardCanAttachToPermanentId(giant)` (not a class the card file names)
    Handler::predicate("BoonweaverGiant.AuraCardCanAttachToGiant", can_attach),
];

/// `AuraCardCanAttachToPermanentId`: an enchantment card whose spell's target could be the source permanent (the
/// filter's source), on the battlefield. Approximation: the engine's possible targets, so shroud on the Giant would
/// also refuse it (Java's `getFilter().match` doesn't look at shroud)
fn can_attach(game: &Game, you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(giant) = source else { return false };
    if game.obj(giant).zone != Zone::Battlefield || !game.obj(id).is(CardType::Enchantment) {
        return false;
    }
    let def = game.obj(id).def.clone();
    def.spell.targets.iter().any(|t| possible_targets(game, you, t, Some(id)).1.contains(&giant))
}

/// `HumanPlayer.choose` of a required one-card target among `cards`: nothing asked with no cards; with none of
/// them choosable, the empty dialog
pub(super) fn choose_one(game: &mut Game, d: &mut dyn Decider, you: Seat, cards: &[ObjId], possible: &[ObjId]) -> Option<Option<ObjId>> {
    if cards.is_empty() {
        return Some(None);
    }
    if possible.is_empty() {
        super::gilgamesh_master_at_arms::choose_any(game, d, you, possible, 1)?;
        return Some(None);
    }
    Some(choose_cards_rounds(game, d, you, possible, 1, 1)?.first().copied())
}

/// (the trigger asked "you may") "Search your graveyard for an Aura card?", then (none yet) "Search your Hand for an
/// Aura card?", then (none yet) the library searched and shuffled; the Aura found enters attached to the Giant
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let on_battlefield = cx.source_on_battlefield(game);
    let handler = serde_json::from_value(serde_json::json!("BoonweaverGiant.AuraCardCanAttachToGiant")).ok()?;
    let filter =
        Filter::of(vec![Pred::Subtype { s: "Aura".into() }, Pred::Card { handler }]);
    let matches = |game: &Game, cards: &[ObjId]| -> Vec<ObjId> {
        if !on_battlefield {
            return Vec::new();
        }
        cards.iter().copied().filter(|c| game.filter_matches(&filter, you, Some(src), *c)).collect()
    };
    let mut card = None;
    if choose_use(game, d, you, "Search your graveyard for an Aura card?")? {
        let cards = game.player(you).graveyard.clone();
        let possible = matches(game, &cards);
        card = choose_one(game, d, you, &cards, &possible)?;
    }
    if card.is_none() && choose_use(game, d, you, "Search your Hand for an Aura card?")? {
        let cards = game.player(you).hand.clone();
        let possible = matches(game, &cards);
        card = choose_one(game, d, you, &cards, &possible)?;
    }
    if card.is_none() {
        // (the Giant gone: `AuraCardCanAttachToPermanentId(null)` matches nothing, as `can_attach` refuses it)
        card = search_library(game, d, you, &filter, 1, 1, src)?.first().copied();
        let mut library = game.player(you).library.clone();
        game.rng.shuffle(&mut library);
        game.player_mut(you).library = library;
    }
    let Some(card) = card else { return Some(()) };
    if on_battlefield {
        game.attach_entering = Some((card, src, game.obj(src).zcc));
    }
    put_onto_battlefield(game, d, card, you);
    game.attach_entering = None;
    if d.stopped() {
        return None;
    }
    Some(())
}
