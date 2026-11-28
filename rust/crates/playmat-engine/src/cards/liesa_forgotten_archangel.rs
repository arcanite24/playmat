//! Liesa, Forgotten Archangel: "Whenever another nontoken creature you control dies, return that card to its owner's
//! hand at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("LiesaForgottenArchangel.LiesaForgottenArchangelReturnToHandEffect", apply)
        .effects(&["new ReturnToHandTargetEffect()"])];

/// `AtTheBeginOfNextEndStepDelayedTriggeredAbility` with a `ReturnToHandTargetEffect` pointed at the card that died
/// (the trigger's `FixedTargets`, while it's still that card)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(c) = cx.obj(0) else { return Some(()) };
    let delayed = Delayed {
        rule: "At the beginning of the next end step, return that card to its owner's hand.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: game.obj(cx.source_id()).zcc,
        fixed: vec![(c, game.obj(c).zcc)],
        return_tapped: Vec::new(),
        targets: Vec::new(),
        condition: None,
    };
    let you = cx.controller();
    game.at_next_end_step.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), you) });
    Some(())
}
