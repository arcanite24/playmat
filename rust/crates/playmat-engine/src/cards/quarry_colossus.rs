//! Quarry Colossus: "When Quarry Colossus enters, put target creature into its owner's library just beneath the top
//! X cards of that library, where X is the number of Plains you control."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("QuarryColossus.QuarryColossusReturnLibraryEffect", apply)];

/// `putCardOnTopXOfLibrary(permanent, X + 1)` for X Plains you control (`PutIntoLibraryNFromTopTargetEffect`: on the
/// bottom when the library is too small)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    if cx.obj(0).is_none_or(|o| game.obj(o).zone != Zone::Battlefield) {
        return Some(());
    }
    let plains = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == you && game.has_subtype(**o, "Plains"))
        .count();
    cx.run(game, d, &[EffectDef::PutIntoLibraryNFromTopTargetEffect { n: plains + 1 }]);
    Some(())
}
