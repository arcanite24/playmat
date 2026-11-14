//! Prying Questions: "Target opponent loses 3 life and puts a card from their hand on top of their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PryingQuestions.PryingQuestionsEffect", apply)];

/// the opponent chooses a card from their hand (`TargetCardInHand()`), put on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, p, &hand, &hand, 1, 1, "to put on top of your library")?;
    if let Some(c) = chosen.first() {
        to_library(game, *c, true);
    }
    Some(())
}
