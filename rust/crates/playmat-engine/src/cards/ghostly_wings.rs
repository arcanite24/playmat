//! Ghostly Wings: "Discard a card: Return enchanted creature to its owner's hand."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GhostlyWings.GhostlyWingsReturnEffect", apply)];

/// `getPermanentOrLKIBattlefield`: the creature this Aura is (or, destroyed in response, last was) attached to
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = game.obj(cx.source_id());
    let (creature, zcc) = if src.zcc == cx.source.1 { src.attached_to } else { src.lki_attached_to }?;
    if game.obj(creature).zone == Zone::Battlefield && game.obj(creature).zcc == zcc {
        game.move_to(creature, Zone::Hand);
    }
    Some(())
}
