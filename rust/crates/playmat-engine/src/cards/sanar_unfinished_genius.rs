//! Sanar, Unfinished Genius: "{T}: Create a Treasure token. Activate only if you've cast an instant or sorcery spell
//! this turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("SanarUnfinishedGenius.SanarUnfinishedGeniusCondition", cast_instant_or_sorcery)];

/// `SpellsCastWatcher`: an instant or sorcery spell you cast this turn
fn cast_instant_or_sorcery(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).instants_sorceries_cast > 0
}
