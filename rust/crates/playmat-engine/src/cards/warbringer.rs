//! Warbringer: "Dash costs you pay cost {2} less (as long as this creature is on the battlefield). Dash {2}{R}"

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("Warbringer.WarbringerSpellsCostReductionEffect", CostKind::Reduce, dash_cost, two_less)];

/// your spell: being cast, cast for its dash cost (`DashedCondition`); while what's playable is checked
/// (`inCheckPlayableState`), any spell of yours, as long as Warbringer's own card has dash (Java looks at the
/// source's card, not the spell's: a Gruul Spellbreaker is offered for {2} less, then asked for in full)
fn dash_cost(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if of.ability.is_some() || of.seat != cx.controller {
        return false;
    }
    if game.stack.iter().any(|i| i.card == of.card && i.ability.is_none()) {
        return game.obj(of.card).dashed;
    }
    game.obj(cx.source)
        .def
        .abilities
        .iter()
        .any(|a| matches!(a, AbilityDef::EvokeAbility { keyword, .. } if keyword == "Dash"))
}

/// `CardUtil.reduceCost(ability, 2)`
fn two_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}
