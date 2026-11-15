//! Anafenza, the Foremost: "Whenever Anafenza, the Foremost attacks, put a +1/+1 counter on another target tapped
//! creature you control. If a nontoken creature an opponent owns would die or a creature card not on the
//! battlefield would be put into an opponent's graveyard, exile that card instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::replacement(
    "AnafenzaTheForemost.AnafenzaTheForemostEffect",
    &[EventType::ZoneChange],
    applies,
    replace,
)];

/// a card an opponent owns going to a graveyard: a creature permanent from the battlefield, or a creature card from
/// anywhere else
fn applies(game: &Game, cx: &StaticCx, ev: &Event) -> bool {
    let Some(id) = ev.target else { return false };
    ev.to == Some(Zone::Graveyard) && cx.opponent(game.obj(id).owner) && game.has_type(id, CardType::Creature)
}

/// exiled instead, except a token leaving the battlefield (Java's `replaceEvent` leaves it be)
fn replace(game: &mut Game, _: &StaticCx, ev: &mut Event) -> bool {
    let Some(id) = ev.target else { return false };
    if ev.from == Some(Zone::Battlefield) && game.obj(id).token {
        return false;
    }
    ev.to = Some(Zone::Exile);
    false
}
