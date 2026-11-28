//! Incinerator of the Guilty: "Whenever Incinerator of the Guilty deals combat damage to a player, you may collect
//! evidence X. When you do, Incinerator of the Guilty deals X damage to each creature and planeswalker that player
//! controls."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("IncineratorOfTheGuilty.IncineratorOfTheGuiltyEffect", apply),
    // the reflexive trigger's `DamageAllControlledTargetEffect(X, creature and planeswalker)` on that player
    Handler::effect("IncineratorOfTheGuilty.IncineratorOfTheGuiltyDamageEffect", damage),
];

/// "Collect evidence X?", `announceX(0, MAX)`, then `CollectEvidenceCost(X).pay` (no `canPay` first: cards chosen
/// from the graveyard, paid if their mana values reach X); paid, the reflexive trigger with X
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if !choose_use(game, d, you, "Collect evidence X?")? {
        return Some(());
    }
    let x = loop {
        match ask(game, d, you, &Query::Amount { min: 0, max: i32::MAX }) {
            Answer::Int(n) if n >= 0 => break n,
            Answer::Stop => return None,
            _ => {}
        }
        if d.stopped() {
            return None;
        }
    };
    let paid = pay_cost(game, d, you, cx.source_id(), &CostDef::CollectEvidenceCost { amount: x });
    if d.stopped() {
        return None;
    }
    if !paid {
        return Some(());
    }
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "CardEffect", "handler": "IncineratorOfTheGuilty.IncineratorOfTheGuiltyDamageEffect"
    }))
    .ok()?;
    let delayed = Delayed {
        rule: format!("{{this}} deals {x} damage to each creature and planeswalker that player controls."),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![e],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    // (the damaged player is the trigger's pointer; in a duel, the controller's opponent, which `damage` reads)
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), x, ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// each creature and planeswalker the damaged player (the controller's opponent: combat damage to a player is to
/// the defending one) controls, in battlefield order, is dealt X by the source
fn damage(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    let player = game.opponent(cx.controller());
    let victims: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|b| {
            game.obj(*b).controller == player
                && (game.has_type(*b, CardType::Creature) || game.has_type(*b, CardType::Planeswalker))
        })
        .collect();
    for v in victims {
        if game.obj(v).zone == Zone::Battlefield {
            deal_damage(game, cx.source_id(), Target::Obj(v), x, false);
        }
    }
    Some(())
}
