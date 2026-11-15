//! Jhoira, Ageless Innovator: "{T}: Put two ingenuity counters on Jhoira, then you may put an artifact card with mana
//! value X or less from your hand onto the battlefield, where X is the number of ingenuity counters on Jhoira."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("JhoiraAgelessInnovator.JhoiraAgelessInnovatorEffect", apply)];

/// X the ingenuity counters on Jhoira (as it last was); with an artifact card of mana value X or less in your hand,
/// "Put an artifact with mana value X or less into play?", then one chosen (`TargetCardInHand`) onto the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game.obj(cx.source_id()).counter("ingenuity");
    let hand = game.player(you).hand.clone();
    let possible: Vec<ObjId> = hand
        .iter()
        .copied()
        .filter(|c| game.obj(*c).is(CardType::Artifact) && (game.obj(*c).def.mana_value() as i32) < x + 1)
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    if !choose_use(game, d, you, &format!("Put an artifact with mana value {x} or less into play?"))? {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    Some(())
}
