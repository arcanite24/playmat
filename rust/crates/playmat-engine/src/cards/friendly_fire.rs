//! Friendly Fire: "Target creature's controller reveals a card at random from their hand. Friendly Fire deals damage
//! to that creature and that player equal to the revealed card's mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FriendlyFire.FriendlyFireEffect", apply)];

/// the creature still there and its controller's hand not empty: `getHand().getRandom(game)` (no random number for a
/// single card), revealed, then its mana value as damage to the creature, then to that player
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let creature = cx.obj(0)?;
    let p = game.obj(creature).controller;
    let hand = game.player(p).hand.clone();
    if hand.is_empty() {
        return Some(());
    }
    let i = if hand.len() < 2 { 0 } else { game.rng.next_int_bound(hand.len() as i32) as usize };
    let card = hand[i];
    let zcc = game.obj(cx.source_id()).zcc;
    game.revealed.push(crate::state::Revealed {
        source: cx.source_id(),
        zcc,
        paren: false,
        cards: vec![(card, Zone::Hand)],
    });
    let damage = game.obj(card).def.mana_value() as i32;
    if damage > 0 {
        deal_damage(game, cx.source_id(), Target::Obj(creature), damage, false);
        deal_damage(game, cx.source_id(), Target::Player(p), damage, false);
    }
    Some(())
}
