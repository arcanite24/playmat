//! Psychic Pickpocket: "When this creature enters, it connives. When it connives this way, return up to one target nonland permanent to its owner's
//! hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PsychicPickpocket.ConniveSourceEffect", apply)
    .args(&["text", "text"])
    .effects(&["new ReturnToHandTargetEffect()"])
    .filters(&["StaticFilters.FILTER_PERMANENT_NON_LAND"])];

/// `ConniveSourceEffect("it", ability)`: it connives (always done, even as last known: `connive` is true), then
/// `fireReflexiveTriggeredAbility` of the bounce, with up to one target nonland permanent
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &[EffectDef::ConniveSourceEffect]);
    if d.stopped() {
        return None;
    }
    let target = TargetDef::TargetPermanent {
        filter: cx.filters.first()?.clone(),
        min: 0,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "return up to one target nonland permanent to its owner's hand.".into(),
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
