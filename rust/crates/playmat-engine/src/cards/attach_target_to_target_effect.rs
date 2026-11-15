//! `AttachTargetToTargetEffect`: "attach target Equipment you control to target attacking creature" (Raubahn, Bull
//! of Ala Mhigo; Halvar, God of Battle): the first target's permanent becomes attached to the second's.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("AttachTargetToTargetEffect", apply)];

/// `EachTargetPointer`: the legal targets of every group, in order; with two or more, the second gets the first
/// (`addAttachment`)
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let permanents: Vec<ObjId> = (0..cx.legal.len())
        .flat_map(|i| cx.objs(i))
        .filter(|o| game.obj(*o).zone == Zone::Battlefield)
        .collect();
    // (already attached to it: nothing happens, as `addAttachment` returns)
    if let [attachment, to, ..] = permanents[..] {
        if game.attached(attachment) == Some(to) {
            return Some(());
        }
        // `cantBeAttachedBy`: protection, and an Aura's enchant restriction (its spell's target; Halvar can't move
        // Skygames from a land onto a creature)
        let a = game.obj(attachment);
        if game.protected_from(to, attachment) {
            return Some(());
        }
        if a.def.has_subtype("Aura") {
            if let Some(TargetDef::TargetPermanent { filter, .. }) = a.def.spell.targets.first() {
                if !game.filter_matches(filter, a.controller, Some(attachment), to) {
                    return Some(());
                }
            }
        }
        let zcc = game.obj(to).zcc;
        game.attach_to(attachment, to, zcc);
    }
    Some(())
}
