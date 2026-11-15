//! Malicious Eclipse: "All creatures get -2/-2 until end of turn. If a creature an opponent controls would die this
//! turn, exile it instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "MaliciousEclipse.MaliciousEclipseReplacementEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)];

/// `isDiesEvent` of a creature an opponent of the spell's controller controls (`FILTER_OPPONENTS_PERMANENT_CREATURE`)
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.from == Some(Zone::Battlefield)
        && ev.to == Some(Zone::Graveyard)
        && game.has_type(id, CardType::Creature)
        && cx.opponent(game.obj(id).controller)
}

/// exiled instead
fn replace(_: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    ev.to = Some(Zone::Exile);
    false
}
