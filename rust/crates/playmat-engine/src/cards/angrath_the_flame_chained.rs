//! Angrath, the Flame-Chained: "+1: Each opponent discards a card and loses 2 life. −3: Gain control of target
//! creature until end of turn. Untap it. It gains haste until end of turn. Sacrifice it at the beginning of the next
//! end step if it has mana value 3 or less. −8: Each opponent loses life equal to the number of cards in their
//! graveyard."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("AngrathTheFlameChained.AngrathTheFlameCreateDelayedTriggerEffect", delayed_sacrifice),
    Handler::condition("AngrathTheFlameChained.AngrathTheFlameChainedDelayedTriggeredAbility", mana_value_3_or_less),
    Handler::effect("AngrathTheFlameChained.AngrathTheFlameUltimateEffect", ultimate),
];

/// With the target still on the battlefield, the delayed trigger at the beginning of the next end step: its
/// controller sacrifices it (`SacrificeTargetEffect("sacrifice this", you)`, a `FixedTarget`) if it has mana value 3
/// or less then
fn delayed_sacrifice(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let Ok(condition) = serde_json::from_value::<Condition>(serde_json::json!({
        "c": "card",
        "handler": "AngrathTheFlameChained.AngrathTheFlameChainedDelayedTriggeredAbility",
    })) else {
        return Some(());
    };
    let delayed = Delayed {
        rule: "Sacrifice it at the beginning of the next end step if it has mana value 3 or less.".into(),
        sacrifice: vec![(o, game.obj(o).zcc)],
        warp_exile: Vec::new(),
        effects: Vec::new(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: Some(condition),
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// `checkTrigger`: the permanent is still the same object, with mana value 3 or less (the waiting or resolving
/// delayed trigger's)
fn mana_value_3_or_less(game: &Game, _: Seat, _: ObjId) -> bool {
    let item = game.checking_item.as_ref().or(game.resolving_item.as_ref());
    let Some((o, zcc)) = item.and_then(|i| i.delayed.as_ref()).and_then(|dl| dl.sacrifice.first().copied()) else {
        return false;
    };
    let p = game.obj(o);
    p.zone == Zone::Battlefield && p.zcc == zcc && p.def.mana_value() <= 3
}

/// Each opponent with cards in their graveyard loses that much life
fn ultimate(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for k in 1..game.seats() {
        let p = (you + k) % game.seats();
        let n = game.player(p).graveyard.len() as i32;
        if n > 0 {
            lose_life(game, p, n);
        }
    }
    Some(())
}
