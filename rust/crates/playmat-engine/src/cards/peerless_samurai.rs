//! Peerless Samurai: "Menace. Whenever a Samurai or Warrior you control attacks alone, the next spell you cast this
//! turn costs {1} less to cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("PeerlessSamurai.PeerlessSamuraiEffect", CostKind::Reduce, next_spell, one_less).on_start(remember)];

/// `CastSpellLastTurnWatcher.getAmountOfSpellsPlayerCastOnCurrentTurn`
fn spells_cast(game: &Game, seat: Seat) -> i32 {
    game.watch.spells.iter().filter(|s| s.caster == seat).count() as i32
}

/// `init`: the spells you've cast so far this turn, kept as the effect's amount
fn remember(game: &mut Game, _: &mut dyn Decider, cx: &StaticCx) -> Option<Vec<(Target, u32)>> {
    let n = spells_cast(game, cx.controller);
    cx.set_left(game, n);
    Some(cx.targets.to_vec())
}

/// until another spell is cast after it started (then it's used up): your spells
fn next_spell(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if spells_cast(game, cx.controller) > cx.left(game, 0) {
        return false;
    }
    of.ability.is_none() && of.seat == cx.controller
}

/// `CardUtil.reduceCost(ability, 1)`
fn one_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
