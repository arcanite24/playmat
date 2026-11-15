//! Stormsplitter: "Whenever you cast an instant or sorcery spell, create a token that's a copy of Stormsplitter.
//! Exile that token at the beginning of the next end step."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Stormsplitter.StormsplitterEffect", apply)];

/// `CreateTokenCopyTargetEffect.setSavedPermanent(source or its last known information)`, then
/// `exileTokensCreatedAtNextEndStep`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let src = cx.source_id();
    let made = token_copy_modified(game, d, src, you, 1, &|_: &mut CardDef| {});
    if d.stopped() {
        return None;
    }
    let made: Vec<ObjId> = made.into_iter().filter(|t| game.obj(*t).zone == Zone::Battlefield).collect();
    remove_tokens_at_next_end_step(game, &made, src, you, true);
    Some(())
}
