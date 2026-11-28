//! Heartwarming Redemption: "Discard all the cards in your hand, then draw that many cards plus one. You gain life
//! equal to the number of cards in your hand."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("HeartwarmingRedemption.HeartwarmingRedemptionEffect", apply)];

/// `discard(hand size)` (the whole hand, nothing asked), draw that many plus one, then life for the hand's size
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let n = game.player(you).hand.len() as i32;
    let discarded = discard(game, d, you, n).len();
    if d.stopped() {
        return None;
    }
    draw(game, you, discarded + 1);
    let life = game.player(you).hand.len() as i32;
    gain_life(game, you, life);
    Some(())
}
