//! Street Sweeper: "Whenever Street Sweeper attacks, destroy all Auras attached to target land."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("StreetSweeper.StreetSweeperDestroyEffect", apply)];

/// Each Aura attached to the land, in attachment order, destroyed
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(land) = cx.obj(0) else { return Some(()) };
    if game.obj(land).zone != Zone::Battlefield {
        return Some(());
    }
    let attached: Vec<ObjId> = game.attachments(land).collect();
    for a in attached {
        if game.obj(a).zone == Zone::Battlefield && game.has_subtype(a, "Aura") {
            destroy(game, a);
        }
    }
    Some(())
}
