//! Glaring Spotlight: "Creatures your opponents control with hexproof can be the targets of spells and abilities you
//! control as though they didn't have hexproof."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("GlaringSpotlight.GlaringSpotlightEffect", AsThough::Hexproof(ignores))];

/// your spells and abilities, at a permanent an opponent controls (Java's test is on any permanent)
fn ignores(game: &Game, cx: &StaticCx, seat: Seat, t: Target) -> bool {
    let Target::Obj(o) = t else { return false };
    seat == cx.controller && game.obj(o).zone == Zone::Battlefield && cx.opponent(game.obj(o).controller)
}
