//! Warbringer: "Dash costs you pay cost {2} less (as long as this creature is on the battlefield). Dash {2}{R}"

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("Warbringer.WarbringerSpellsCostReductionEffect", CostKind::Reduce, dash_cost, two_less)];

/// your spell: being cast, cast for its dash cost (`DashedCondition`); while what's playable is checked
/// (`inCheckPlayableState`), any spell of a card with dash
fn dash_cost(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if of.ability.is_some() || of.seat != cx.controller {
        return false;
    }
    if game.stack.iter().any(|i| i.card == of.card && i.ability.is_none()) {
        return game.obj(of.card).dashed;
    }
    game.obj(of.card)
        .def
        .abilities
        .iter()
        .any(|a| matches!(a, AbilityDef::EvokeAbility { keyword, .. } if keyword == "Dash"))
}

/// `CardUtil.reduceCost(ability, 2)`
fn two_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}
