//! Ran and Shaw: "Flying. Firebending 2. When Ran and Shaw enter, if you cast them and there are three or more
//! Dragon and/or Lesson cards in your graveyard, create a token that's a copy of Ran and Shaw, except it's not
//! legendary. {3}{R}: Dragons you control get +2/+0 until end of turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RanAndShaw.RanAndShawEffect", copy)];

/// `CreateTokenCopyTargetEffect().setIsntLegendary(true)` of the source as it is, or as it last was on the
/// battlefield (`getSourcePermanentOrLKI`)
fn copy(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if !matches!(game.obj(src).zone, Zone::Battlefield | Zone::Graveyard | Zone::Exile) {
        return Some(());
    }
    token_copy_modified(game, d, src, cx.controller(), 1, &|def: &mut CardDef| {
        def.supertypes.retain(|s| s != "LEGENDARY");
    });
    (!d.stopped()).then_some(())
}
