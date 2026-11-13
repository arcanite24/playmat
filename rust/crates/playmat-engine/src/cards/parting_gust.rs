//! Parting Gust: "Exile target nontoken creature. If the gift wasn't promised, return that creature to the
//! battlefield under its owner's control with a +1/+1 counter on it at the beginning of the next end step."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("PartingGust.PartingGustExileReturnEffect", apply),
    // the delayed trigger's `ReturnToBattlefieldUnderOwnerControlWithCounterTargetEffect` (no class of the card's)
    Handler::effect("PartingGust.PartingGustReturnWithCounter", return_with_counter),
];

/// The target is exiled, then "at the beginning of the next end step, return that card …" points at the card
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let gone: Vec<ObjId> = cx.objs(0).into_iter().filter(|o| game.obj(*o).zone == Zone::Battlefield).collect();
    if gone.is_empty() {
        return Some(());
    }
    for o in &gone {
        game.move_to(*o, Zone::Exile);
    }
    let fixed: Vec<(ObjId, u32)> = gone
        .iter()
        .filter(|o| game.obj(**o).zone == Zone::Exile && !game.obj(**o).token)
        .map(|o| (*o, game.obj(*o).zcc))
        .collect();
    let back: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CardEffect", "handler": "PartingGust.PartingGustReturnWithCounter"}))
            .ok()?;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return that card to the battlefield under its owner's control."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![back],
        source_zcc: cx.source.1,
        fixed,
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The cards still in exile return under their owner's control, each with a +1/+1 counter
fn return_with_counter(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards = cx.objs(0);
    cx.run(game, d, &[EffectDef::ReturnToBattlefieldTargetEffect { your_control: false, tapped: false }]);
    for c in cards {
        if game.obj(c).zone == Zone::Battlefield {
            add_counters_by(game, c, "+1/+1", 1, Some(cx.controller()));
        }
    }
    Some(())
}
