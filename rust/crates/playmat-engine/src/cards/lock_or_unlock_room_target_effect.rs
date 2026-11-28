//! `LockOrUnlockRoomTargetEffect`: "lock or unlock a door of target Room you control" (Marina Vendrell).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LockOrUnlockRoomTargetEffect", apply)];

/// Java's four questions by which doors are unlocked (yes: the left door), then `unlockDoor` or `lockDoor`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(room) = cx.obj(0).filter(|o| game.obj(*o).zone == Zone::Battlefield) else { return Some(()) };
    let [left, right] = game.obj(room).doors;
    let (message, lock_if_left, lock_if_right) = match (left, right) {
        (false, false) => ("Unlock the left door or the right door?", false, false),
        (false, true) => ("Unlock the left door or lock the right door?", false, true),
        (true, false) => ("Lock the left door or unlock the right door?", true, false),
        (true, true) => ("Lock the left door or the right door?", true, true),
    };
    let use_left = choose_use(game, d, you, message)?;
    let lock = if use_left { lock_if_left } else { lock_if_right };
    let k = if use_left { 0 } else { 1 };
    if lock {
        if game.obj(room).doors[k] {
            game.obj_mut(room).doors[k] = false;
            set_room_face(game, room);
        }
    } else {
        unlock_door(game, room, k);
    }
    Some(())
}
