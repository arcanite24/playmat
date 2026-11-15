//! Hardened Berserker: "Whenever Hardened Berserker attacks, the next spell you cast this turn costs {1} less to
//! cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "HardenedBerserker.HardenedBerserkerSpellsCostReductionEffect",
    CostKind::Reduce,
    applies,
    reduce,
)
.on_start(start)];

/// `init`: the spells you've cast this turn so far (`CastSpellLastTurnWatcher`), kept as the effect's amount
fn start(game: &mut Game, _: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let cast = game.player(cx.controller).spells_cast as i32;
    cx.set_left(game, cast);
    Some(cx.targets.to_vec())
}

/// a spell you cast, until you've cast another spell this turn (then it's used up)
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let cast_before = cx.left(game, 0);
    if game.player(cx.controller).spells_cast as i32 > cast_before {
        return false;
    }
    of.ability.is_none() && of.seat == cx.controller
}

/// `CardUtil.reduceCost(ability, 1)`
fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
