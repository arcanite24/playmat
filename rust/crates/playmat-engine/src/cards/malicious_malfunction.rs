//! Malicious Malfunction: "All creatures get -2/-2 until end of turn. If a creature would die this turn, exile it
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MaliciousMalfunction.MaliciousMalfunctionReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)];

/// `isDiesEvent` of a creature: from the battlefield to a graveyard
fn applies(game: &Game, _: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard) && game.has_type(id, CardType::Creature)
}

fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
