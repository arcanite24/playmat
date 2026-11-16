//! Bladegraft Aspirant: "Menace. Equipment spells you cast cost {1} less to cast. Activated abilities of Equipment
//! you control that target Bladegraft Aspirant cost {1} less to activate."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "BladegraftAspirant.BladegraftAspirantCostReductionEffect",
    CostKind::Reduce,
    targets_me,
    one_less,
)];

/// an activated ability of an Equipment you control; being activated, the Aspirant among its chosen targets
/// (`getAllSelectedTargets`); while what's playable is checked, among its possible targets (`getAllPossibleTargets`)
fn targets_me(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let Some(a) = of.ability else { return false };
    let Some((_, _, targets, _, _)) = a.activated() else { return false };
    if !game.has_subtype(of.card, "Equipment") || game.obj(of.card).controller != cx.controller {
        return false;
    }
    if let Some(item) = game.stack.iter().rev().find(|i| i.card == of.card && i.ability.is_some()) {
        return item.targets.iter().flatten().any(|t| *t == Target::Obj(cx.source));
    }
    targets.iter().any(|def| possible_targets(game, of.seat, def, Some(of.card)).1.contains(&cx.source))
}

/// `CardUtil.reduceCost(ability, 1)` of the generic mana there is
fn one_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
