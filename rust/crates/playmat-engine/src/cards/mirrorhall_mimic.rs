//! Mirrorhall Mimic // Ghastly Mimicry: "At the beginning of your upkeep, create a token that's a copy of enchanted
//! creature, except it's a Spirit in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MirrorhallMimic.GhastlyMimicryEffect", apply)];

/// The creature the Aura (or its last known information) is attached to, still on the battlefield: a token copy of
/// it, a Spirit in addition
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let o = game.obj(cx.source_id());
    let Some((attached, _)) = o.attached_to.or(o.lki_attached_to) else { return Some(()) };
    if game.obj(attached).zone != Zone::Battlefield {
        return Some(());
    }
    token_copy_modified(game, d, attached, you, 1, &|def: &mut CardDef| {
        let creature = def.types.contains(&CardType::Creature) || def.types.contains(&CardType::Kindred);
        if creature && !def.subtypes.iter().any(|s| s == "Spirit") {
            def.subtypes.push("Spirit".into());
        }
    });
    if d.stopped() {
        return None;
    }
    Some(())
}
