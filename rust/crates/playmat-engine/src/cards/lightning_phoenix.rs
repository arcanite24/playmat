//! Lightning Phoenix: "At the beginning of your end step, if an opponent was dealt 3 or more damage this turn, you may
//! pay {R}. If you do, return Lightning Phoenix from your graveyard to the battlefield."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("LightningPhoenix.LightningPhoenixCondition", dealt_three)];

/// `LightningPhoenixWatcher`: the damage dealt to your opponents this turn (`DAMAGED_PLAYER`, every event's amount)
/// adds up to 3 or more
fn dealt_three(game: &Game, you: Seat, _: ObjId) -> bool {
    let dealt: i32 = game
        .watch
        .damage
        .iter()
        .filter(|d| matches!(d.to, Target::Player(p) if p != you))
        .map(|d| d.amount)
        .sum();
    dealt >= 3
}
