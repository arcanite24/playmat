//! Kaya, Bane of the Dead: "Your opponents and permanents your opponents control with hexproof can be the targets of
//! spells and abilities you control as though they didn't have hexproof. −3: Exile target creature."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("KayaBaneOfTheDead.KayaBaneOfTheDeadEffect", AsThough::Hexproof(applies))];

/// your spells and abilities, at an opponent or a permanent an opponent controls (`game.getPermanent`)
fn applies(game: &Game, cx: &StaticCx, seat: Seat, t: Target) -> bool {
    seat == cx.controller
        && match t {
            Target::Player(p) => cx.opponent(p),
            Target::Obj(o) => game.obj(o).zone == Zone::Battlefield && cx.opponent(game.obj(o).controller),
        }
}
