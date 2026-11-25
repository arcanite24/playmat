//! Purphoros, Bronze-Blooded: "{2}{R}: You may put a red creature card or an artifact creature card from your hand
//! onto the battlefield. Sacrifice it at the beginning of the next end step."

use super::*;
use crate::card::PutCards;

pub const HANDLERS: &[Handler] = &[Handler::effect("PurphorosBronzeBlooded.PurphurosBronzeBloodedEffect", apply)];

/// "Put a red creature card or an artifact creature card from your hand onto the battlefield?", then a
/// `TargetCardInHand(filter)` chosen and put onto the battlefield, with a delayed "sacrifice it" for the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let message = "Put a red creature card or an artifact creature card from your hand onto the battlefield?";
    if !choose_use(game, d, you, message)? {
        return Some(());
    }
    let hand = game.player(you).hand.clone();
    let possible: Vec<ObjId> = hand
        .iter()
        .copied()
        .filter(|c| {
            game.has_type(*c, CardType::Creature)
                && (game.has_type(*c, CardType::Artifact) || game.colors(*c).contains(&Color::R))
        })
        .collect();
    if possible.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(card) = choose_cards(game, d, you, &hand, &possible, 1, 1, &rule)?.first().copied() else {
        return Some(());
    };
    if !put_cards(game, d, you, &[card], PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let delayed = Delayed {
        rule: format!("At the beginning of the next end step, sacrifice {}.", game.obj(card).def.name),
        sacrifice: vec![(card, game.obj(card).zcc)],
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
