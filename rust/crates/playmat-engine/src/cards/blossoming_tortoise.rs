//! Blossoming Tortoise: "Activated abilities of lands you control cost {1} less to activate."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "BlossomingTortoise.BlossomingTortoiseCostReductionEffect",
    CostKind::Reduce,
    applies,
    reduce,
)];

/// an activated ability of a land you control on the battlefield
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let o = game.obj(of.card);
    of.ability.is_some() && o.zone == Zone::Battlefield && o.controller == cx.controller && game.has_type(of.card, CardType::Land)
}

/// `CardUtil.reduceCost(ability, 1)`
fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
