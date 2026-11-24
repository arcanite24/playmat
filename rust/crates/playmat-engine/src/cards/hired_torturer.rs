//! Hired Torturer: "{3}{B}, {T}: Target opponent loses 2 life, then reveals a card at random from their hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("HiredTorturer.HiredTorturerEffect", apply)];

/// `getHand().getRandom(game)` (no random number for a single card), revealed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let i = if hand.len() < 2 { 0 } else { game.rng.next_int_bound(hand.len() as i32) as usize };
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(hand[i], Zone::Hand)],
    });
    Some(())
}
