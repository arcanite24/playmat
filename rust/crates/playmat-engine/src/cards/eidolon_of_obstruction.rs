//! Eidolon of Obstruction: "First strike. Loyalty abilities of planeswalkers your opponents control cost {1} more to
//! activate."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("EidolonOfObstruction.EidolonOfObstructionEffect", CostKind::Increase, applies, more)];

/// a loyalty ability of a planeswalker, activated by an opponent
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    matches!(of.ability, Some(AbilityDef::LoyaltyAbility { .. }))
        && game.obj(of.card).zone == Zone::Battlefield
        && game.has_type(of.card, CardType::Planeswalker)
        && of.seat != cx.controller
}

/// `CardUtil.increaseCost(abilityToModify, 1)`
fn more(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 1;
}
