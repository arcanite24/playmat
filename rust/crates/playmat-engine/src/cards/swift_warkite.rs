//! Swift Warkite: "Flying. When Swift Warkite enters, you may put a creature card with mana value 3 or less from your
//! hand or graveyard onto the battlefield. That creature gains haste. Return it to your hand at the beginning of the
//! next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SwiftWarkite.SwiftWarkiteEffect", apply)];

/// The creature cards of mana value 3 or less in your hand, then your graveyard; with any, up to one chosen (not
/// targeted) onto the battlefield, with haste and a delayed "return it to your hand" at the next end step
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let fits = |game: &Game, c: ObjId| game.obj(c).is(CardType::Creature) && game.obj(c).def.mana_value() < 4;
    let mut cards: Vec<ObjId> = game.player(you).hand.iter().copied().filter(|c| fits(game, *c)).collect();
    cards.extend(game.player(you).graveyard.iter().copied().filter(|c| fits(game, *c)));
    if cards.is_empty() {
        return Some(());
    }
    let rule = item_rule(game, cx.item);
    let Some(&c) = choose_cards(game, d, you, &cards, &cards, 0, 1, &rule)?.first() else { return Some(()) };
    if !put_cards(game, d, you, &[c], crate::card::PutCards::Battlefield) {
        return None;
    }
    if game.obj(c).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(c).zcc;
    Arc::make_mut(&mut game.grants).push(crate::state::Grant {
        target: c,
        zcc,
        ability: AbilityDef::HasteAbility,
        until_end_of_turn: false,
        until_turn_of: None,
    });
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return it to your hand.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ReturnToHandTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(c, zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
