//! Moment of Truth: "Look at the top three cards of your library. Put one of those cards into your hand, one into
//! your graveyard, and one on the bottom of your library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MomentOfTruth.MomentOfTruthEffect", apply)];

/// One for the hand (required with three cards), one for the graveyard (required with two left), the last to the
/// bottom
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let mut cards: Vec<ObjId> = game.player(you).library.iter().take(3).copied().collect();
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let min = usize::from(cards.len() == 3);
    if let Some(&c) = choose_cards(game, d, you, &cards, &cards, min, 1, &rule)?.first() {
        game.move_to(c, Zone::Hand);
        cards.retain(|x| *x != c);
    }
    if cards.is_empty() {
        return Some(());
    }
    let min = usize::from(cards.len() == 2);
    if let Some(&c) = choose_cards(game, d, you, &cards, &cards, min, 1, &rule)?.first() {
        game.move_to(c, Zone::Graveyard);
        cards.retain(|x| *x != c);
    }
    if cards.is_empty() {
        return Some(());
    }
    let Some(&c) = choose_cards(game, d, you, &cards, &cards, 1, 1, &rule)?.first() else { return Some(()) };
    if !put_on_library(game, d, you, &[c], false, true) {
        return None;
    }
    Some(())
}
