//! Inferno of the Star Mounts: "{R}: Inferno of the Star Mounts gets +1/+0 until end of turn. When its power becomes
//! 20 this way, it deals 20 damage to any target."

use super::*;
use crate::card::TargetDef;

pub const HANDLERS: &[Handler] = &[Handler::effect("InfernoOfTheStarMounts.InfernoOfTheStarMountsEffect", apply)
    .effects(&["new DamageTargetEffect(20)"])];

/// its power (while it's still the permanent) is 20 after the boost: `fireReflexiveTriggeredAbility`, a reflexive
/// trigger dealing 20 damage to any target waits to go on the stack
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) || game.power_toughness(cx.source_id()).0 != 20 {
        return Some(());
    }
    let delayed = Delayed {
        rule: "When its power becomes 20 this way, it deals 20 damage to any target.".into(),
        sacrifice: Vec::new(),
        warp_exile: Vec::new(),
        effects: cx.effects.to_vec(),
        source_zcc: cx.source.1,
        fixed: Vec::new(),
        return_tapped: Vec::new(),
        targets: vec![TargetDef::TargetAnyTarget {
            min: 1,
            max: 1,
            another: false,
            not_first: false,
            distribute: None,
            x_count: false,
            count: None,
        }],
        condition: None,
    };
    game.pending.push(StackItem { delayed: Some(Arc::new(delayed)), ..StackItem::spell(cx.source_id(), cx.controller()) });
    Some(())
}
