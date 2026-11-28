//! Vaultborn Tyrant: "When Vaultborn Tyrant dies, if it's not a token, create a token that's a copy of it, except
//! it's an artifact in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("VaultbornTyrant.VaultbornTyrantCreateCopyEffect", apply)];

/// `CreateTokenCopyTargetEffect(you, ARTIFACT, false)` of the Tyrant as it last was (`getSourcePermanentOrLKI`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    token_copy_modified(game, d, cx.source_id(), you, 1, &|def: &mut CardDef| {
        if !def.types.contains(&CardType::Artifact) {
            def.types.push(CardType::Artifact);
        }
    });
    if d.stopped() {
        return None;
    }
    Some(())
}
