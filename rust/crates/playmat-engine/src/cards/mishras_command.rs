//! Mishra's Command: "Choose target player. They may discard up to X cards. Then they draw a card for each card
//! discarded this way."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MishrasCommand.MishrasCommandEffect", apply)];

/// `player.discard(0, X, ...)`: a `TargetDiscard` of zero to X (nothing asked at X 0), then the draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let x = cx.item.x;
    if x <= 0 {
        return Some(());
    }
    let hand = game.player(p).hand.clone();
    // `getToDiscard`: a hand no bigger than the least (none) is all of it, unasked
    if hand.is_empty() {
        return Some(());
    }
    let chosen = choose_cards(game, d, p, &hand, &hand, 0, x as usize, "discard")?;
    for c in &chosen {
        discard_card(game, p, *c);
    }
    if !chosen.is_empty() {
        game.discarded_cards();
    }
    draw(game, p, chosen.len());
    Some(())
}
