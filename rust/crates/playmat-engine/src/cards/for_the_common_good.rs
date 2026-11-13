//! For the Common Good: "Create X tokens that are copies of target token you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ForTheCommonGood.ForTheCommonGoodEffect", apply)];

/// X copies (`CreateTokenCopyTargetEffect(null, null, false, X)` of the target), when X is more than 0 and the
/// target is still there
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x;
    let token = cx.obj(0)?;
    if x <= 0 || game.obj(token).zone != Zone::Battlefield {
        return Some(());
    }
    let e: EffectDef =
        serde_json::from_value(serde_json::json!({"class": "CreateTokenCopyTargetEffect", "number": x})).ok()?;
    cx.run(game, d, &[e]);
    Some(())
}
