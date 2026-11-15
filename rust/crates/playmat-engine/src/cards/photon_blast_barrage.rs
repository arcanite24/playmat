//! Photon Blast Barrage: "When you cast this spell, copy it X times. You may choose new targets for the copies.
//! Photon Blast Barrage deals 1 damage to target creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PhotonBlastBarrage.PhotonBlastBarrageCopyEffect", apply)];

/// The spell, still on the stack as the one cast (`getValue("spellCast")`), copied X times (the X it was cast with),
/// each copy offering new targets (`createCopyOnStack(…, true, X)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (spell, zcc) = cx.source;
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    if game.obj(spell).zcc != zcc {
        return Some(());
    }
    for _ in 0..original.x.max(0) {
        copy_spell(game, d, &original, you);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
