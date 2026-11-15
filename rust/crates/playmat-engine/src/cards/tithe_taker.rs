//! Tithe Taker: "During your turn, spells your opponents cast cost {1} more to cast and abilities your opponents
//! activate cost {1} more to activate unless they're mana abilities."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("TitheTaker.TitheTakerCostReductionEffect", CostKind::Increase, applies, more)];

/// your turn, and an opponent's spell or activated ability that isn't a mana ability
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    game.active == Some(cx.controller) && of.seat != cx.controller && of.ability.is_none_or(|a| a.mana().is_none())
}

fn more(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 1;
}
