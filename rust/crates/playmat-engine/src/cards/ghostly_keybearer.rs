//! Ghostly Keybearer: "Flying. Whenever this creature deals combat damage to a player, unlock a locked door of up to
//! one target Room you control."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GhostlyKeybearer.GhostlyKeybearerEffect", apply)];

/// The target Room with a locked door: the only locked one, or "Unlock the left door or the right door?" (Left is
/// yes) with both locked, then `unlockDoor`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let id = cx.obj(0)?;
    if game.obj(id).zone != Zone::Battlefield {
        return Some(());
    }
    let [left, right] = game.obj(id).doors;
    if left && right {
        return Some(());
    }
    let unlock_left = if !left && right {
        true
    } else if left && !right {
        false
    } else {
        choose_use(game, d, cx.controller(), "Unlock the left door or the right door?")?
    };
    unlock_door(game, id, if unlock_left { 0 } else { 1 });
    Some(())
}
