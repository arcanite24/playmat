//! `PutCardIntoPlayWithHasteAndSacrificeEffect(filter, Duration.Custom, …)`: "you may put an artifact card from your
//! hand onto the battlefield. That artifact gains haste. Sacrifice it at the beginning of the next end step." (Arms
//! Race, Meek Attack). Only the `Duration.Custom` haste, which lasts while it stays on the battlefield.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PutCardIntoPlayWithHasteAndSacrificeEffect", apply).args(&[
    "filter",
    "one_of:Duration.Custom",
    "text",
    "text",
])];

/// `player.choose` of a `TargetCardInHand(0, 1, filter)`; the card put onto the battlefield, haste for as long as
/// it's there, and the delayed "sacrifice it" at the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let hand = game.player(you).hand.clone();
    let possible: Vec<ObjId> =
        hand.iter().copied().filter(|c| game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), *c)).collect();
    let rule = item_rule(game, cx.item);
    let chosen = choose_cards(game, d, you, &hand, &possible, 0, 1, &rule)?;
    let Some(&card) = chosen.first() else { return Some(()) };
    if game.obj(card).zone != Zone::Hand {
        return Some(());
    }
    if !put_cards(game, d, you, &[card], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(card).zcc;
    Arc::make_mut(&mut game.grants).push(crate::state::Grant {
        target: card,
        zcc,
        ability: AbilityDef::HasteAbility,
        until_end_of_turn: false,
        until_turn_of: None,
    });
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, sacrifice it.".into(),
        sacrifice: vec![(card, zcc)],
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
