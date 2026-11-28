//! Black Sun's Twilight: "Up to one target creature gets -X/-X until end of turn. If X is 5 or more, return a
//! creature card with mana value X or less from your graveyard to the battlefield tapped."
//!
//! The return is upstream's `OneShotNonTargetEffect` with an `XManaValueTargetAdjuster` (the transpiler's), inside a
//! `ConditionalOneShotEffect` on this condition.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::condition("BlackSunsTwilight.BlackSunsTwilightCondition", x_five_or_more)];

/// `CardUtil.getSourceCostsTag(game, source, "X", 0) >= 5`: the resolving spell's X
fn x_five_or_more(game: &Game, _: Seat, _: ObjId) -> bool {
    game.resolving_item.as_ref().is_some_and(|i| i.x >= 5)
}
