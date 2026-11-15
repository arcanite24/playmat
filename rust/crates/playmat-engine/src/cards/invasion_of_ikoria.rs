//! Invasion of Ikoria: "When Invasion of Ikoria enters the battlefield, search your library and/or graveyard for a
//! non-Human creature card with mana value X or less and put it onto the battlefield. If you search your library
//! this way, shuffle." Zilortha, Apex of Ikoria: "For each non-Human creature you control, you may have that
//! creature assign its combat damage as though it weren't blocked."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::predicate("InvasionOfIkoria.InvasionOfIkoriaPredicate", at_most_x),
    Handler::as_though("InvasionOfIkoria.ZilorthaApexOfIkoriaEffect", AsThough::DamageNotBlocked(non_human)),
];

/// Its mana value is at most the X the source was cast with (`GetXValue`: the permanent's cost tags, 0 when it
/// wasn't cast); false with no source
fn at_most_x(game: &Game, _you: Seat, source: Option<ObjId>, id: ObjId) -> bool {
    let Some(src) = source else { return false };
    let x = if game.obj(src).zone == Zone::Battlefield { game.obj(src).cast_x } else { 0 };
    (game.obj(id).def.mana_value() as i32) <= x
}

/// `applies`: a non-Human attacker you control, if you say yes to the engine's question
fn non_human(game: &Game, cx: &StaticCx, attacker: ObjId) -> Option<bool> {
    (game.obj(attacker).controller == cx.controller && !game.has_subtype(attacker, "Human")).then_some(true)
}
