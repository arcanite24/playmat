//! Flaying Tendrils: "Devoid. All creatures get -2/-2 until end of turn. If a creature would die this turn, exile it
//! instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "FlayingTendrils.FlayingTendrilsReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)];

/// `isDiesEvent` of a creature (resolved: until the turn's cleanup, `Duration.EndOfTurn`)
fn applies(game: &Game, _cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield) && ev.to == Some(Zone::Graveyard) && game.has_type(id, CardType::Creature)
}

/// exiled instead
fn replace(_game: &mut Game, _cx: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
