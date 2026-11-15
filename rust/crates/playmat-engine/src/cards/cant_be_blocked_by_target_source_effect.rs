//! `CantBeBlockedByTargetSourceEffect(Duration.EndOfTurn)`: "{this} can't be blocked by target creature this turn"
//! (Kozilek's Pathfinder, Fearsome Temper's granted ability): resolving, the source by the target creature.

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("CantBeBlockedByTargetSourceEffect", applies, &[Restrict::CantBeBlocked(by)])
        .args(&["one_of:Duration.EndOfTurn"])];

fn applies(_: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    attacker == cx.source
}

fn by(game: &Game, cx: &StaticCx, _: ObjId, blocker: ObjId) -> bool {
    cx.target_obj(game, 0) == Some(blocker)
}
