//! Mysterious Limousine: "Whenever Mysterious Limousine enters or attacks, exile up to one other target creature until
//! Mysterious Limousine leaves the battlefield. If a creature is put into exile this way, return each other card
//! exiled with Mysterious Limousine to the battlefield under its owner's control. Crew 2"

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MysteriousLimousine.MysteriousLimousineEffect", apply)];

/// With the target still a permanent and this still on the battlefield: what's already in this one's exile zone is
/// noted, the target joins it (back when this leaves, `OnLeaveReturnExiledAbility`), then the noted cards return to
/// the battlefield under their owners' control
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    if game.obj(target).zone != Zone::Battlefield || !cx.source_on_battlefield(game) {
        return Some(());
    }
    let (src, zcc) = cx.source;
    let before: Vec<ObjId> = game
        .exiled_until_leaves
        .iter()
        .filter(|(s, z, c)| *s == src && *z == zcc && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| *c)
        .collect();
    game.move_to(target, Zone::Exile);
    if game.obj(target).zone == Zone::Exile {
        game.exiled_until_leaves.push((src, zcc, target));
    }
    if before.is_empty() {
        return Some(());
    }
    game.exiled_until_leaves.retain(|(s, z, c)| !(*s == src && *z == zcc && before.contains(c)));
    for c in before {
        if game.obj(c).zone != Zone::Exile {
            continue;
        }
        let owner = game.obj(c).owner;
        put_onto_battlefield(game, d, c, owner);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
