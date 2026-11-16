//! Earth Rumble: "Earthbend 2. When you do, up to one target creature you control fights target creature an opponent
//! controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EarthRumble.EarthRumbleEffect", apply)
    .effects(&["new FightTargetsEffect()"])
    .filters(&["StaticFilters.FILTER_CONTROLLED_CREATURE", "StaticFilters.FILTER_OPPONENTS_PERMANENT_CREATURE"])];

/// `fireReflexiveTriggeredAbility`, always (the earthbend before it doesn't gate it): a reflexive trigger with up to
/// one creature you control and a creature an opponent controls as targets, waiting to go on the stack
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let target = |filter: &Filter, min| TargetDef::TargetPermanent {
        filter: filter.clone(),
        min,
        max: 1,
        distribute: None,
        x_count: false,
        count: None,
    };
    let delayed = Delayed {
        rule: "target creature you control fights target creature an opponent controls. <i>(Each deals damage equal \
               to its power to the other.)</i>"
            .into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![target(&cx.filters[0], 0), target(&cx.filters[1], 1)],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
