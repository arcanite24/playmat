//! Koya, Death from Above: "Flying. When Koya enters, exile up to one other target creature. At the beginning of the
//! next end step, you may pay {3}{B}. If you don't, return that card to the battlefield under its owner's control."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("KoyaDeathFromAbove.KoyaDeathFromAboveEffect", apply),
    Handler::effect(RETURN, return_it),
];

/// the delayed trigger's "if you don't" (`ReturnToBattlefieldUnderOwnerControlTargetEffect(false, true)`)
const RETURN: &str = "KoyaDeathFromAbove.KoyaDeathFromAboveReturnEffect";

/// The target still on the battlefield exiled; then the delayed "you may pay {3}{B}; if you don't, return it" at the
/// next end step (`DoIfCostPaid(null, returnEffect, {3}{B})`), pointing at the card while it's still that card in
/// exile (a token exiled is gone)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let creatures: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if creatures.is_empty() {
        return Some(());
    }
    for o in &creatures {
        game.move_to(*o, Zone::Exile);
    }
    let fixed: Vec<(ObjId, u32)> = creatures
        .iter()
        .filter(|o| game.obj(**o).zone == Zone::Exile && !game.obj(**o).token)
        .map(|o| (*o, game.obj(*o).zcc))
        .collect();
    let effect: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "DoIfCostPaid",
        "cost": [{"class": "ManaCost", "cost": "{3}{B}"}],
        "otherwise": [{"class": "CardEffect", "handler": RETURN}]
    }))
    .ok()?;
    let delayed = crate::state::Delayed {
        rule: "At the beginning of the next end step, you may pay {3}{B}. If you don't, return that card to the \
               battlefield under its owner's control."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![effect],
        source_zcc: cx.source.1,
        fixed,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// The exiled cards (the delayed trigger's fixed targets, still those cards in exile) onto the battlefield, each
/// under its owner's control
fn return_it(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    for c in cx.objs(0) {
        if game.obj(c).zone != Zone::Exile {
            continue;
        }
        let owner = game.obj(c).owner;
        if !put_cards(game, d, owner, &[c], crate::card::PutCards::Battlefield) {
            return None;
        }
    }
    Some(())
}
