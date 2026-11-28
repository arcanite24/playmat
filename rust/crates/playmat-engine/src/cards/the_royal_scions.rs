//! The Royal Scions: "+1: Draw a card, then discard a card. +1: Target creature gets +2/+0 and gains first strike and
//! trample until end of turn. −8: Draw four cards. When you do, The Royal Scions deals damage to any target equal to
//! the number of cards in your hand."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("TheRoyalScions.TheRoyalScionsCreateReflexiveTriggerEffect", apply).effects(&[
        "new DrawCardSourceControllerEffect(4)",
        "new DamageTargetEffect(CardsInControllerHandCount.ANY)",
    ])];

/// The four cards, then `fireReflexiveTriggeredAbility` (always: Java doesn't check the draw), its target chosen as
/// it goes on the stack and the damage counted as it resolves
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let target: TargetDef = serde_json::from_value(serde_json::json!({"class": "TargetAnyTarget"})).ok()?;
    let delayed = Delayed {
        rule: "{this} deals damage to any target equal to the number of cards in your hand.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects[1..2].to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
