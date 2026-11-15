//! `CantBeBlockedByAllSourceEffect(filterBlockedBy, duration)`: "{this} can't be blocked by creatures with power 2
//! or less this turn" (Rhonas's Stalwart, resolving): the source, while it's the same permanent, by blockers of the
//! filter (from the source's controller).

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("CantBeBlockedByAllSourceEffect", applies, &[Restrict::CantBeBlocked(by)])
        .args(&["filter", "one_of:Duration.EndOfTurn|Duration.WhileOnBattlefield"])];

fn applies(_: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    attacker == cx.source
}

fn by(game: &Game, cx: &StaticCx, _: ObjId, blocker: ObjId) -> bool {
    cx.matches(game, 0, blocker)
}
