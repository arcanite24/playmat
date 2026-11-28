//! Bone Picker: "This spell costs {3} less to cast if a creature died this turn. Flying, deathtouch."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("BonePicker.BonePickerAdjustingCostsEffect", CostKind::Reduce, applies, reduce).in_any_zone()];

/// its own spell, when a creature died this turn (`MorbidCondition`)
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.card == cx.source && game.creature_died
}

/// `CardUtil.reduceCost(ability, 3)`: three generic
fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(3);
}
