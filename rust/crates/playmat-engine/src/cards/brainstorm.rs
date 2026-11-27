//! `BrainstormEffect` (Cavalier of Gales): "draw three cards, then put two cards from your hand on top of your library
//! in any order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BrainstormEffect", apply)];

/// two `TargetCardInHand` choices, one card at a time, each put on top
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let draw_n = cx.arg_int(0).unwrap_or(3).max(0) as usize;
    let put_n = cx.arg_int(1).unwrap_or(2).max(0) as usize;
    draw(game, you, draw_n);
    let rule = item_rule(game, cx.item);
    for _ in 0..put_n {
        let hand = game.player(you).hand.clone();
        if hand.is_empty() {
            break;
        }
        let Some(&c) = choose_cards(game, d, you, &hand, &hand, 1, 1, &rule)?.first() else { break };
        to_library(game, c, true);
    }
    Some(())
}
