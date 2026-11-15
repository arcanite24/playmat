//! Burrog Barrage: "if you've cast another instant or sorcery spell this turn" (`SpellsCastWatcher`): an instant or
//! sorcery you cast this turn from a card other than this one. Barrage itself is an instant you cast, so that's a
//! second one.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BurrogBarrage.BurrogBarrageCondition", another_cast)];

fn another_cast(game: &Game, you: Seat, _source: ObjId) -> bool {
    game.player(you).instants_sorceries_cast >= 2
}
