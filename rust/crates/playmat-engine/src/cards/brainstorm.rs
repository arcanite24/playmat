//! `BrainstormEffect` (Cavalier of Gales): "draw three cards, then put two cards from your hand on top of your library
//! in any order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrainstormEffect", apply)];

/// two `TargetCardInHand` choices, one card at a time, each put on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 3);
    let rule = item_rule(game, cx.item);
    for _ in 0..2 {
        let hand = game.player(you).hand.clone();
        if hand.is_empty() {
            break;
        }
        let Some(&c) = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first() else { break };
        to_library(game, c, true);
    }
    Some(())
}
