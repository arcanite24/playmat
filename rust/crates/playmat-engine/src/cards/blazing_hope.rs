//! Blazing Hope: "Exile target creature with power greater than or equal to your life total."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::target(
    "BlazingHope.BlazingHopeTarget",
    "new TargetCreaturePermanent()",
    power_at_least_life,
)];

/// `canTarget` (and `canChoose`): its power at least your life total
fn power_at_least_life(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    game.power_toughness(id).0 >= game.player(you).life
}
