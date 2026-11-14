//! The Notary Hobbits: "When The Notary Hobbits enter, if they're not a token, create two tokens that are copies of
//! them, except the tokens aren't legendary."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TheNotaryHobbits.TheNotaryHobbitsEffect", apply)];

/// `CreateTokenCopyTargetEffect(you, null, false, 2).setSavedPermanent(the permanent that entered)`, not legendary
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let of = cx.item.entering.unwrap_or(cx.source_id());
    token_copy_modified(game, d, of, you, 2, &|def: &mut CardDef| def.supertypes.retain(|s| s != "LEGENDARY"));
    if d.stopped() {
        return None;
    }
    Some(())
}
