//! Sentinel of the Pearl Trident: "Flash. When Sentinel of the Pearl Trident enters, you may exile target historic
//! permanent you control. If you do, return that card to the battlefield under its owner's control at the beginning
//! of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SentinelOfThePearlTrident.SentinelOfThePearlTridentEffect", apply)];

/// The target exiled, then "at the beginning of the next end step, return that card to the battlefield under its
/// owner's control" (`ReturnToBattlefieldUnderOwnerControlTargetEffect(false, false)` on the card as it was exiled)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e: EffectDef = serde_json::from_value(serde_json::json!({
        "class": "ExileReturnBattlefieldNextEndStepTargetEffect", "your_control": false, "tapped": false}))
    .ok()?;
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
