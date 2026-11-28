//! War Historian: "Reach. War Historian has indestructible as long as it attacked a battle this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("WarHistorian.WarHistorianCondition", attacked_a_battle)];

/// `WarHistorianWatcher`: it was declared attacking a battle this turn. The engine has no battles, so it never was.
fn attacked_a_battle(_: &Game, _: Seat, _: ObjId) -> bool {
    false
}
