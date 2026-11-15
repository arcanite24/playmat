//! Stormwing Entity: "This spell costs {2}{U} less to cast if you've cast an instant or sorcery spell this turn."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("StormwingEntity.StormwingEntityCondition", applies)];

/// an instant or sorcery spell you cast this turn (`SpellsCastWatcher`; this creature spell isn't one)
fn applies(game: &Game, you: Seat, _: ObjId) -> bool {
    game.player(you).instants_sorceries_cast > 0
}
