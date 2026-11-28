//! Kili the Resourceful: "As long as you have an enduring story, you may pay {0} rather than pay the equip cost of the
//! first equip ability you activate each turn."

use super::*;
use crate::card::{AbilityDef, Condition, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("KiliTheResourceful.KiliTheResourcefulCostEffect", CostKind::MayFree, first_equip, free)];

fn first_equip(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.seat == cx.controller
        && matches!(of.ability, Some(AbilityDef::EquipAbility { .. }))
        && game.condition_holds(Some(&Condition::EnduringStory), cx.controller, cx.source)
        && !game.watch.equip_activated.contains(&of.seat)
}

fn free(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    *cost = ManaCost::default();
}
