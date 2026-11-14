//! `DestroyAllAttachedToTargetEffect(filter, description)` (Turn to Slag): "destroy all <filter> attached to
//! <description>".

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DestroyAllAttachedToTargetEffect", apply).args(&["filter", "text"])];

/// The target's attachments of the filter, in the order they were attached, destroyed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(Target::Obj(t)) = cx.item.targets.first().and_then(|g| g.first()).copied() else { return Some(()) };
    let you = cx.controller();
    let attached: Vec<ObjId> = game.attachments(t).collect();
    for a in attached {
        if game.obj(a).zone == Zone::Battlefield && game.filter_matches(&cx.filters[0], you, Some(cx.source_id()), a) {
            destroy(game, a);
        }
    }
    Some(())
}
