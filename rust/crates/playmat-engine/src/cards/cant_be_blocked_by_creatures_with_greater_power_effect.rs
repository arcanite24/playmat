//! `CantBeBlockedByCreaturesWithGreaterPowerEffect`: "Creatures with power greater than {this}'s power can't block
//! it." (Silumgar Assassin)

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::restriction(
    "CantBeBlockedByCreaturesWithGreaterPowerEffect",
    applies,
    &[Restrict::CantBeBlocked(by)],
)];

fn applies(_: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    attacker == cx.source
}

fn by(game: &Game, _: &StaticCx, attacker: ObjId, blocker: ObjId) -> bool {
    game.power_toughness(blocker).0 > game.power_toughness(attacker).0
}
