//! Flame Sweep: "Flame Sweep deals 2 damage to each creature except for creatures you control with flying."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::predicate("FlameSweep.FlameSweepPredicate", not_your_flier)];

fn not_your_flier(game: &Game, you: Seat, _source: Option<ObjId>, id: ObjId) -> bool {
    !(game.obj(id).controller == you && game.has_ability(id, &AbilityDef::FlyingAbility))
}
