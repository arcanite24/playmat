//! Teferi's Time Twist: "Exile target permanent you control. Return that card to the battlefield under its owner's
//! control at the beginning of the next end step. If it enters the battlefield as a creature, it enters with an
//! additional +1/+1 counter on it."

use super::*;
use crate::state::Delayed;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("TeferisTimeTwist.TeferisTimeTwistEffect", apply),
    Handler::effect("TeferisTimeTwist.TeferisTimeTwistReturnEffect", return_it),
];

/// The target exiled (`moveCards`); once it moved, "at the beginning of the next end step, return the exiled card
/// to the battlefield", pointing at the card in exile (`MageObjectReference(id, zcc + 1)`: a token's points at
/// nothing, its trigger still goes on the stack)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Battlefield {
        return Some(());
    }
    let token = game.obj(o).token;
    game.move_to(o, Zone::Exile);
    if game.obj(o).zone != Zone::Exile && !token {
        return Some(());
    }
    let back: EffectDef = serde_json::from_value(
        serde_json::json!({"class": "CardEffect", "handler": "TeferisTimeTwist.TeferisTimeTwistReturnEffect"}),
    )
    .ok()?;
    let fixed = if token { Vec::new() } else { vec![(o, game.obj(o).zcc)] };
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return the exiled card to the battlefield.".into(),
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

/// The card, while it's still in exile, onto the battlefield under its owner's control; a creature there then gets
/// a +1/+1 counter (Java adds it after it enters)
fn return_it(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let cards: Vec<ObjId> = cx.objs(0).into_iter().filter(|c| game.obj(*c).zone == Zone::Exile).collect();
    if cards.is_empty() {
        return Some(());
    }
    cx.run(game, d, &[EffectDef::ReturnToBattlefieldTargetEffect { your_control: false, tapped: false }]);
    for c in cards {
        if game.obj(c).zone == Zone::Battlefield && game.has_type(c, CardType::Creature) {
            add_counters_by(game, c, "+1/+1", 1, Some(cx.controller()));
        }
    }
    Some(())
}
