//! Ruric Thar, Magecrusher: "This spell can't be countered. Reach, vigilance, trample. Ruric Thar has hexproof as
//! long as they haven't dealt combat damage yet."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::condition("RuricTharMagecrusher.RuricTharMagecrusherCondition", no_combat_damage_yet)];

/// `RuricTharMagecrusherWatcher.checkCreature`: this permanent (as the object it is now) hasn't dealt combat damage
fn no_combat_damage_yet(game: &Game, _: Seat, source: ObjId) -> bool {
    !game.combat_damage_sources.contains(&(source, game.obj(source).zcc))
}
