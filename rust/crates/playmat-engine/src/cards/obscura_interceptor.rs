//! Obscura Interceptor: "When this creature enters, it connives. When it connives this way, return up to one target spell to its owner's
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ObscuraInterceptor.ConniveSourceEffect", apply)
    .args(&["text", "text"])
    .effects(&["new ReturnToHandTargetEffect()"])];

/// `ConniveSourceEffect("it", ability)`: it connives (always done, even as last known: `connive` is true), then
/// `fireReflexiveTriggeredAbility` of the bounce, with up to one target spell
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &[EffectDef::ConniveSourceEffect]);
    if d.stopped() {
        return None;
    }
    let target = TargetDef::TargetSpell { filter: crate::card::Filter(Vec::new()), min: 0, max: 1 };
    let delayed = Delayed {
        rule: "return up to one target spell to its owner's hand.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
