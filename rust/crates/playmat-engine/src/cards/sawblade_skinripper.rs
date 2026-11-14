//! Sawblade Skinripper: "Menace. {2}, Sacrifice another creature or enchantment: Put a +1/+1 counter on Sawblade
//! Skinripper. At the beginning of your end step, if you sacrificed one or more permanents this turn, Sawblade
//! Skinripper deals that much damage to any target."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::value("SawbladeSkinripper.SawbladeSkinripperValue", sacrificed),
    Handler::condition("SawbladeSkinripper.SawbladeSkinripperCondition", any_sacrificed),
];

/// `PermanentsSacrificedWatcher`: the permanents you sacrificed this turn
fn sacrificed(game: &Game, you: Seat, _: ObjId, _x: i32) -> i32 {
    game.watch.sacrificed.iter().filter(|(p, _)| *p == you).count() as i32
}

fn any_sacrificed(game: &Game, you: Seat, source: ObjId) -> bool {
    sacrificed(game, you, source, 0) > 0
}
