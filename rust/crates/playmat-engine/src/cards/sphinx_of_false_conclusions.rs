//! Sphinx of False Conclusions: "When this creature dies, if it isn't a token, create a token that's a copy of it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SphinxOfFalseConclusions.SphinxOfFalseConclusionsEffect", apply)];

/// `CreateTokenCopyTargetEffect(useLKI = true)` of the creature that died (the trigger's card, else its source)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let of = cx.obj(0).unwrap_or(cx.source_id());
    token_copy(game, d, of, cx.controller(), None);
    if d.stopped() {
        return None;
    }
    Some(())
}
