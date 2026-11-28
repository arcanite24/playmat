//! `ExileReturnBattlefieldOwnerNextEndStepSourceEffect` (Nezahal, Primal Tide): "exile {this}. Return it to the
//! battlefield [tapped] under its owner's control at the beginning of the next end step"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ExileReturnBattlefieldOwnerNextEndStepSourceEffect", apply)];

/// The source, if it's still the permanent that made the ability, is exiled and returns as
/// `ExileReturnBattlefieldNextEndStepTargetEffect` returns its targets
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if !cx.source_on_battlefield(game) {
        return Some(());
    }
    let tapped = cx.args.first().is_some_and(|a| a == "true");
    let e = EffectDef::ExileReturnBattlefieldNextEndStepTargetEffect { your_control: false, tapped };
    cx.run_on(game, d, &[e], vec![Target::Obj(cx.source_id())]);
    Some(())
}
