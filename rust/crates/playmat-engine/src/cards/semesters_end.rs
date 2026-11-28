//! Semester's End: "Exile any number of target creatures and/or planeswalkers you control. At the beginning of the
//! next end step, return each of them to the battlefield under its owner's control. Each of them enters the
//! battlefield with an additional +1/+1 counter on it if it's a creature and an additional loyalty counter on it if
//! it's a planeswalker."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("SemestersEnd.SemestersEndEffect", apply),
    Handler::effect("SemestersEnd.SemestersEndReturnEffect", back),
];

/// The targets into exile together, then an "at the beginning of the next end step" return of those that are
/// there (`AtTheBeginOfNextEndStepDelayedTriggeredAbility`, its cards as `MageObjectReference`s)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards: Vec<ObjId> = cx.objs(0);
    for c in &cards {
        game.move_to(*c, Zone::Exile);
    }
    // (a token that left the battlefield ceased to exist: it doesn't come back)
    let exiled: Vec<(ObjId, u32)> = cards
        .iter()
        .filter(|c| game.obj(**c).zone == Zone::Exile && !game.obj(**c).token)
        .map(|c| (*c, game.obj(*c).zcc))
        .collect();
    let back: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "SemestersEnd.SemestersEndReturnEffect"}))
            .ok()?;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return the exiled card to the battlefield.".into(),
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

/// The cards still in exile onto the battlefield together, each under its owner's control, then a +1/+1 counter on
/// each creature and a loyalty counter on each planeswalker among them, put by you
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
        if game.obj(c).zone != Zone::Battlefield {
            continue;
        }
        if game.has_type(c, CardType::Creature) {
            put_counters(game, c, "+1/+1", 1, you);
        }
        if game.has_type(c, CardType::Planeswalker) {
            put_counters(game, c, "loyalty", 1, you);
        }
    }
    Some(())
}
