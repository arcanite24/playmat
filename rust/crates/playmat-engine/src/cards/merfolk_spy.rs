//! Merfolk Spy: "Whenever Merfolk Spy deals combat damage to a player, that player reveals a card at random from their
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MerfolkSpy.MerfolkSpyEffect", apply)];

/// `getHand().getRandom(game)` (the hand in order, no random number for a single card), then `revealCards` titled
/// "Merfolk Spy"
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(0) else { return Some(()) };
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
