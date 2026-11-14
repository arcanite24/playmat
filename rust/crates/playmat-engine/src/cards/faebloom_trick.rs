//! Faebloom Trick: "Create two 1/1 blue Faerie creature tokens with flying. When you do, tap target creature an
//! opponent controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FaebloomTrick.FaebloomTrickEffect", apply)
    .effects(&["new CreateTokenEffect(new FaerieToken(), 2)", "new TapTargetEffect()"])
    .filters(&["StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE"])];

/// The tokens, then `fireReflexiveTriggeredAbility`: a reflexive trigger with its own target, waiting to go on the
/// stack (as `DoWhenCostPaid`'s)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    let target = TargetDef::TargetPermanent {
        filter: cx.filters[0].clone(),
        min: 1,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "tap target creature an opponent controls.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects[1..].to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
