//! Ilharg, the Raze-Boar: "Whenever Ilharg attacks, you may put a creature card from your hand onto the battlefield
//! tapped and attacking. Return that creature to your hand at the beginning of the next end step."

use super::*;
use crate::state::{Delayed, Zone};

pub const HANDLERS: &[Handler] = &[Handler::effect("IlhargTheRazeBoar.IlhargTheRazeBoarEffect", apply)];

/// up to one creature card from your hand (`TargetCardInHand(0, 1, creature)`), put onto the battlefield tapped,
/// then attacking (`addAttackingCreature`); a delayed trigger returns it to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let rule = item_rule(game, cx.item);
    let hand = game.player(you).hand.clone();
    let creatures: Vec<ObjId> = hand.iter().copied().filter(|c| game.has_type(*c, CardType::Creature)).collect();
    let chosen = choose_cards(game, d, you, &hand, &creatures, 0, 1, &rule)?;
    let Some(&card) = chosen.first() else { return Some(()) };
    put_cards(game, d, you, &[card], crate::card::PutCards::BattlefieldTapped);
    if d.stopped() {
        return None;
    }
    if game.obj(card).zone != Zone::Battlefield {
        return Some(());
    }
    if game.has_type(card, CardType::Creature) {
        let defender = game.opponent(you);
        let pw = choose_defender(game, d, you, defender)?;
        let o = game.obj_mut(card);
        o.attacking = Some(defender);
        o.attacking_pw = pw;
        join_combat(game, card);
    }
    let name = game.obj(card).def.name.clone();
    let delayed = Delayed {
        rule: format!("At the beginning of the next end step, return {name} to its owner's hand."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ReturnToHandTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(card, game.obj(card).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
