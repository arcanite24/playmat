//! Detection Tower: "{1}, {T}: Until end of turn, your opponents and creatures with hexproof they control can be the
//! targets of spells and abilities you control as though they didn't have hexproof."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::as_though("DetectionTower.DetectionTowerEffect", AsThough::Hexproof(applies))];

/// your spells and abilities, at an opponent or a permanent an opponent controls (Java checks any permanent)
fn applies(game: &Game, cx: &StaticCx, seat: Seat, t: Target) -> bool {
    seat == cx.controller
        && match t {
            Target::Player(p) => cx.opponent(p),
            Target::Obj(o) => cx.opponent(game.obj(o).controller),
        }
}
