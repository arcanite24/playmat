//! The Ancient One: "Descend 8 — The Ancient One can't attack or block unless there are eight or more permanent
//! cards in your graveyard. {2}{U}{B}: Draw a card, then discard a card. When you discard a card this way, target
//! player mills cards equal to its mana value."

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheAncientOne.TheAncientOneEffect", apply)];

/// A card drawn, then `discardOne`; a card discarded (a land too), the reflexive "target player mills cards equal
/// to its mana value" (`MillCardsTargetEffect(amount)`, `TargetPlayer`) waits for the stack
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    draw(game, you, 1);
    let discarded = discard(game, d, you, 1);
    if d.stopped() {
        return None;
    }
    let Some(&card) = discarded.first() else { return Some(()) };
    let amount = game.obj(card).def.mana_value() as i32;
    let delayed = Delayed {
        rule: "When you discard a card this way, target player mills cards equal to its mana value.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::MillCardsTargetEffect { amount: Amount::Fixed(amount) }],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetPlayer { min: 1, max: 1 }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
