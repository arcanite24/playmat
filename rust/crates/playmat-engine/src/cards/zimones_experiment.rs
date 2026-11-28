//! Zimone's Experiment: "Look at the top five cards of your library. You may reveal up to two creature and/or land
//! cards from among them, then put the rest on the bottom of your library in a random order. Put all land cards
//! revealed this way onto the battlefield tapped and put all creature cards revealed this way into your hand."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("ZimonesExperiment.ZimonesExperimentEffect", apply)];

/// `LookLibraryAndPickControllerEffect(5, 2, creature and/or land cards, HAND, BOTTOM_RANDOM)`, optional and up to
/// two, as the engine's own does it; its `actionWithPickedCards` puts the rest on the bottom first, then the picked
/// lands onto the battlefield tapped and the picked creatures into the hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let source = cx.source_id();
    let cards: Vec<ObjId> = game.player(you).library.iter().take(5).copied().collect();
    if !cards.is_empty() {
        let zcc = game.obj(source).zcc;
        let look = crate::state::LookedAt { viewer: you, source, zcc, bare: false, cards: cards.clone() };
        game.looked_at.push(look);
    }
    let fitting: Vec<ObjId> = cards
        .iter()
        .copied()
        .filter(|c| game.has_type(*c, CardType::Creature) || game.has_type(*c, CardType::Land))
        .collect();
    let number = fitting.len().min(2);
    let mut chosen = Vec::new();
    if number >= 1 {
        let message = "Reveal creature and/or land cards and put them into your hand?";
        if choose_use(game, d, you, message)? {
            let rule = item_rule(game, cx.item);
            chosen = choose_cards(game, d, you, &cards, &fitting, 0, number, &rule)?;
        }
    }
    if !chosen.is_empty() {
        let zcc = game.obj(source).zcc;
        let shown = chosen.iter().map(|c| (*c, Zone::Library)).collect();
        game.revealed.push(crate::state::Revealed { source, zcc, paren: false, cards: shown });
    }
    let rest: Vec<ObjId> = cards.iter().copied().filter(|c| !chosen.contains(c)).collect();
    if !put_cards(game, d, you, &rest, PutCards::BottomRandom) {
        return None;
    }
    let (lands, creatures): (Vec<ObjId>, Vec<ObjId>) =
        chosen.iter().copied().partition(|c| game.has_type(*c, CardType::Land));
    if !lands.is_empty() && !put_cards(game, d, you, &lands, PutCards::BattlefieldTapped) {
        return None;
    }
    if !creatures.is_empty() && !put_cards(game, d, you, &creatures, PutCards::Hand) {
        return None;
    }
    Some(())
}
