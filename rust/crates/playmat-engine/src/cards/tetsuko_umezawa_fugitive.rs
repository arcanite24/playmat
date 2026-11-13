//! Tetsuko Umezawa, Fugitive: "Creatures you control with power or toughness 1 or less can't be blocked." A
//! `CantBeBlockedAllEffect(filter, Duration.Custom)` in a static ability: while Tetsuko is on the battlefield, the
//! filter looked at each time.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "TetsukoUmezawaFugitive.CantBeBlockedAllEffect",
    applies,
    &[Restrict::CantBeBlocked(always)],
)
.args(&["filter", "one_of:Duration.Custom"])];

fn applies(game: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    cx.matches(game, 0, attacker)
}

fn always(_: &Game, _: &StaticCx, _: ObjId, _: ObjId) -> bool {
    true
}
