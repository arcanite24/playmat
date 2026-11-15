//! Long Road Home: "Exile target creature. At the beginning of the next end step, return that card to the
//! battlefield under its owner's control with a +1/+1 counter on it."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("LongRoadHome.LongRoadHomeEffect", apply),
    // the delayed trigger's `ReturnMORToBattlefieldUnderOwnerControlWithCounterEffect` (no class of the card's)
    Handler::effect("LongRoadHome.LongRoadHomeReturnWithCounter", return_with_counter),
];

/// The target is exiled; a card (not a token) in exile gets "at the beginning of the next end step, return that
/// card …", pointing at it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    game.move_to(o, Zone::Exile);
    if game.obj(o).zone != Zone::Exile || game.obj(o).token {
        return Some(());
    }
    let back: EffectDef = serde_json::from_value(
        serde_json::json!({"class": "CardEffect", "handler": "LongRoadHome.LongRoadHomeReturnWithCounter"}),
    )
    .ok()?;
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return that card to the battlefield under its owner's control \
               with a +1/+1 counter on it."
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: vec![back],
        source_zcc: cx.source.1,
        fixed: vec![(o, game.obj(o).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    game.at_next_end_step
        .push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}

/// The card, while it's still in exile, returns under its owner's control with a +1/+1 counter
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
