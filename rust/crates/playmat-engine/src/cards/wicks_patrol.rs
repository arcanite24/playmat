//! Wick's Patrol: "When Wick's Patrol enters, mill three cards. When you do, target creature an opponent controls
//! gets -X/-X until end of turn, where X is the greatest mana value among cards in your graveyard."

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("WicksPatrol.WicksPatrolEffect", apply),
    // (the reflexive trigger's `BoostTargetEffect(WicksPatrolValue, WicksPatrolValue)`, X as it resolves)
    Handler::effect("WicksPatrol.WicksPatrolBoostEffect", boost),
];

/// Mill three; with three milled (fewer: no reflexive trigger, as the release notes say), a reflexive trigger
/// pointing at a creature an opponent controls waits to go on the stack
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let milled = game.player(you).library.len().min(3);
    let mill: EffectDef = serde_json::from_value(serde_json::json!({"class": "MillCardsControllerEffect", "amount": 3})).ok()?;
    cx.run(game, d, &[mill]);
    if d.stopped() {
        return None;
    }
    if milled < 3 {
        return Some(());
    }
    let target: TargetDef = serde_json::from_value(serde_json::json!({
        "class": "TargetPermanent",
        "filter": [{"p": "type", "t": "CREATURE"}, {"p": "controller", "who": "opponent"}],
    }))
    .ok()?;
    let boost: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "WicksPatrol.WicksPatrolBoostEffect"}))
            .ok()?;
    let delayed = Delayed {
        rule: "When you do, target creature an opponent controls gets -X/-X until end of turn, where X is the greatest \
               mana value among cards in your graveyard."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![boost],
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}

/// -X/-X until end of turn, X the greatest mana value among the cards in your graveyard now
fn boost(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = game.player(you).graveyard.iter().map(|c| game.obj(*c).def.mana_value() as i32).max().unwrap_or(0);
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "BoostTargetEffect", "duration": "EndOfTurn", "power": -x, "toughness": -x,
    }))
    .ok()?;
    cx.run(game, d, &[e]);
    Some(())
}
