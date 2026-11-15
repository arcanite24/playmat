//! Serpent of Yawning Depths: "Krakens, Leviathans, Octopuses, and Serpents you control can't be blocked except by
//! Krakens, Leviathans, Octopuses, and Serpents."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "SerpentOfYawningDepths.SerpentOfYawningDepthsEffect",
    sea_creature_you_control,
    &[Restrict::CantBeBlocked(not_by_sea_creature)],
)];

const KINDS: [&str; 4] = ["Kraken", "Leviathan", "Octopus", "Serpent"];

/// a creature that's a Kraken, Leviathan, Octopus or Serpent
fn sea_creature(game: &Game, id: ObjId) -> bool {
    game.has_type(id, CardType::Creature) && KINDS.iter().any(|k| game.has_subtype(id, k))
}

fn sea_creature_you_control(game: &Game, cx: &StaticCx, id: ObjId) -> bool {
    game.obj(id).controller == cx.controller && sea_creature(game, id)
}

/// `canBeBlocked` is the blocker matching the filter: it can't be blocked by anything else
fn not_by_sea_creature(game: &Game, _: &StaticCx, _attacker: ObjId, blocker: ObjId) -> bool {
    !sea_creature(game, blocker)
}
