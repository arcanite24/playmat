//! Speedball, New Warrior: "Whenever a player casts a spell that targets Speedball, he gets +2/+2 until end of turn
//! and you may choose new targets for that spell."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::predicate("SpeedballNewWarrior.SpeedballNewWarriorPredicate", targets_source)];

/// the source is on the battlefield and the spell has it among its chosen targets
fn targets_source(game: &Game, _: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(s) = source.filter(|s| game.obj(*s).zone == Zone::Battlefield) else { return false };
    game.stack
        .iter()
        .filter(|i| i.card == id && i.ability.is_none() && i.delayed.is_none())
        .any(|i| i.targets.iter().flatten().any(|t| *t == Target::Obj(s)))
}
