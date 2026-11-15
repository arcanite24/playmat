//! Zirda, the Dawnwaker: "Companion — Each permanent card in your starting deck has an activated ability. Abilities
//! you activate that aren't mana abilities cost {2} less to activate. This effect can't reduce the mana in that cost
//! to less than one mana. {1}, {T}: Target creature can't block this turn."

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("ZirdaTheDawnwaker.ZirdaTheDawnwakerEffect", CostKind::Reduce, yours, two_less_not_below_one)];

/// `isNonManaActivatedAbility` (a Room's unlock is a special action) you activate
fn yours(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_some_and(|a| !matches!(a, AbilityDef::RoomUnlockAbility { .. })) && of.seat == cx.controller
}

/// `CardUtil.calculateActualPossibleGenericManaReduction(mana, 2, 1)`: up to {2} of the generic mana, leaving at
/// least one mana in the cost
fn two_less_not_below_one(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let non_generic = cost.colored.iter().sum::<u32>()
        + cost.colorless
        + cost.phyrexian.iter().sum::<u32>()
        + cost.hybrid.len() as u32
        + cost.mono.len() as u32
        + cost.mono_half.len() as u32
        + cost.snow;
    let not_possible = 1u32.saturating_sub(non_generic);
    let r = cost.generic.saturating_sub(not_possible).min(2);
    cost.generic -= r;
}
