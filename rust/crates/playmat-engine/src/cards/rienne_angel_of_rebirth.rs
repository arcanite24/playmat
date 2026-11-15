//! Rienne, Angel of Rebirth: "Flying. Other multicolored creatures you control get +1/+0. Whenever another
//! multicolored creature you control dies, return it to its owner's hand at the beginning of the next end step."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "RienneAngelOfRebirth.RienneAngelOfRebirthTriggeredAbility",
        "new DiesCreatureTriggeredAbility(new RienneAngelOfRebirthEffect(), false, filter, true)",
        another_yours,
    ),
    Handler::effect("RienneAngelOfRebirth.RienneAngelOfRebirthEffect", apply),
];

/// `checkTrigger` beyond a multicolored creature dying (the card's filter, on its last known information): another
/// one, its controller the ability's
fn another_yours(_: &Game, item: &StackItem) -> bool {
    item.died.is_some_and(|(o, seat)| seat == item.controller && o != item.card)
}

/// The card, if it's in a graveyard, gets `AtTheBeginOfNextEndStepDelayedTriggeredAbility(
/// ReturnFromGraveyardToHandTargetEffect)` on it there (`FixedTarget(card, game)`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let card = match cx.item.died {
        Some((o, _)) => o,
        None => match cx.item.targets.first().and_then(|ts| ts.first()) {
            Some(Target::Obj(o)) => *o,
            _ => return Some(()),
        },
    };
    if game.obj(card).zone != Zone::Graveyard {
        return Some(());
    }
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return that card to your hand at the beginning of the next end step."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![EffectDef::ReturnToHandTargetEffect],
        source_zcc: cx.source.1,
        fixed: vec![(card, game.obj(card).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
