//! Salvation Swan: "Flash. Flying. Whenever Salvation Swan or another Bird you control enters, exile up to one target
//! creature you control without flying. Return it to the battlefield under its owner's control with a flying counter
//! on it at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SalvationSwan.SalvationSwanTargetEffect", apply),
    Handler::effect("SalvationSwan.SalvationSwanReturnEffect", back),
];

/// The target (if it's still there) into exile, then, even when nothing stayed in exile, an "at the beginning of the
/// next end step" return of what's there (`AtTheBeginOfNextEndStepDelayedTriggeredAbility`, `FixedTargets`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    if game.obj(c).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(c, Zone::Exile);
    // (a token that left the battlefield ceased to exist: it doesn't come back)
    let exiled: Vec<(ObjId, u32)> = [c]
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Exile && !game.obj(**c).token)
        .map(|c| (*c, game.obj(*c).zcc))
        .collect();
    let back: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "SalvationSwan.SalvationSwanReturnEffect"}))
            .ok()?;
    let delayed = Delayed {
        // (the return effect's static text, which leaves out the counter)
        rule: "At the beginning of the next end step, return that card to the battlefield under its owner's control."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![back],
        source_zcc: cx.source.1,
        fixed: exiled,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// `ReturnToBattlefieldUnderOwnerControlWithCounterTargetEffect(FLYING)`: the cards still in exile onto the
/// battlefield together, each under its owner's control, each with a flying counter
fn back(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let cards: Vec<ObjId> =
        cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Exile && !game.obj(*c).token).collect();
    if cards.is_empty() {
        return Some(());
    }
    game.entering_together = cards.clone();
    for c in &cards {
        let owner = game.obj(*c).owner;
        enter_battlefield(game, d, *c, owner, false);
    }
    game.entering_together.clear();
    if d.stopped() {
        return None;
    }
    for c in cards {
        // a double-faced card or an Aura enters as something the counters (by the card's id) don't find
        let faced = game.obj(c).def.transform_back.is_some()
            || game.obj(c).def.back.is_some()
            || game.obj(c).def.subtypes.iter().any(|s| s == "Aura");
        if game.obj(c).zone == Zone::Battlefield && !faced {
            add_counters_by(game, c, "flying", 1, Some(you));
        }
    }
    Some(())
}
