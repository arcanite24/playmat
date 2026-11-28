//! `CantBeBlockedByCreaturesAllEffect(filterCreatures, filterBlockedBy, WhileOnBattlefield)`: "creatures you control
//! with power 2 or less can't be blocked by creatures with power 3 or greater" (Delney, Streetwise Lookout; Graaz,
//! Unstoppable Juggernaut). Both filters are looked at each time, from the source's controller.

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::restriction("CantBeBlockedByCreaturesAllEffect", applies, &[Restrict::CantBeBlocked(by)]).args(&[
        "filter",
        "filter",
        "while_on_battlefield",
    ])];

fn applies(game: &Game, cx: &StaticCx, attacker: ObjId) -> bool {
    cx.matches(game, 0, attacker)
}

fn by(game: &Game, cx: &StaticCx, _: ObjId, blocker: ObjId) -> bool {
    cx.matches(game, 1, blocker)
}
