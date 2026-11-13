//! Dragon Whelp: "{R}: Dragon Whelp gets +1/+0 until end of turn. If this ability has been activated four or more
//! times this turn, sacrifice Dragon Whelp at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("DragonWhelp.DragonWhelpEffect", apply)];

/// `ActivationInfo` counts this effect's resolutions this turn for the source as it is (the engine's
/// `abilities_resolved`, this one included); from the fourth, a delayed "sacrifice {this}" each time
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::IfAbilityHasResolvedXTimesEffect {
        n: 4,
        or_more: true,
        effects: vec![EffectDef::CreateNextEndStepDelayedEffect {
            effects: vec![EffectDef::SacrificeSourceEffect],
            rule: "At the beginning of the next end step, sacrifice {this}.".into(),
        }],
    };
    cx.run(game, d, &[e]);
    Some(())
}
