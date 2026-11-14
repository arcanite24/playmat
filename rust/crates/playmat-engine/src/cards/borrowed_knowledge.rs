//! Borrowed Knowledge: "Choose one — • Discard your hand, then draw cards equal to the number of cards in target
//! opponent's hand. • …"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BorrowedKnowledge.BorrowedKnowledgeEffect", apply)];

/// `player.drawCards(opponent.getHand().size())`
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(opponent) = cx.player(0) else { return Some(()) };
    let n = game.player(opponent).hand.len();
    draw(game, cx.controller(), n);
    Some(())
}
