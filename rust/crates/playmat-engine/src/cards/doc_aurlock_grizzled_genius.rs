//! Doc Aurlock, Grizzled Genius: "Spells you cast from your graveyard or from exile cost {2} less to cast. Plotting
//! cards from your hand costs {2} less."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "DocAurlockGrizzledGenius.DocAurlockGrizzledGeniusEffect",
    CostKind::Reduce,
    applies,
    reduce,
)];

/// your `PlotAbility`, of a card in your hand (`apply` checks the zone)
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.seat == cx.controller
        && of.ability.is_some_and(|a| matches!(a, AbilityDef::PlotAbility { .. }))
        && game.obj(of.card).zone == Zone::Hand
}

/// up to {2} of the generic mana
fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}
