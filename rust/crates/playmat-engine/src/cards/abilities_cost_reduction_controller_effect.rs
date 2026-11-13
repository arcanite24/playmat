//! `AbilitiesCostReductionControllerEffect(XAbility.class, name[, amount[, excludeSource]])`: "ninjutsu abilities
//! you activate cost {1} less to activate" (Silver-Fur Master), "unlock costs you pay cost {1} less" (Inquisitive
//! Glimmer), "power-up abilities of other creatures you control cost {3} less" (Hulk, Gamma Goliath). A reduction
//! (`CardUtil.reduceCost`: generic mana) of the abilities of that class its controller activates.

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "AbilitiesCostReductionControllerEffect",
    CostKind::Reduce,
    applies,
    reduce,
)
.args(&["one_of:NinjutsuAbility.class|RoomUnlockAbility.class|PowerUpAbility.class|EquipAbility.class", "text", "text", "text"])];

/// the activated ability classes this reads, as `XAbility.class`
fn of_class(class: &str, a: &AbilityDef) -> bool {
    match class {
        "NinjutsuAbility.class" => matches!(a, AbilityDef::NinjutsuAbility { .. }),
        "RoomUnlockAbility.class" => matches!(a, AbilityDef::RoomUnlockAbility { .. }),
        "PowerUpAbility.class" => matches!(a, AbilityDef::PowerUpAbility { .. }),
        "EquipAbility.class" => matches!(a, AbilityDef::EquipAbility { .. }),
        _ => false,
    }
}

fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.seat == cx.controller
        && of.ability.is_some_and(|a| cx.args.first().is_some_and(|c| of_class(c, a)))
        // `excludeSource` is checked in `apply`, as Java does
        && !(cx.arg_bool(3) && of.card == cx.source)
}

fn reduce(_: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(cx.arg_int(2).unwrap_or(1).max(0) as u32);
}
