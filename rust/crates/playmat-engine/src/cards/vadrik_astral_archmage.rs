//! Vadrik, Astral Archmage: "If it's neither day nor night, it becomes day as Vadrik, Astral Archmage enters.
//! Instant and sorcery spells you cast cost {X} less to cast, where X is Vadrik's power. Whenever day becomes night
//! or night becomes day, put a +1/+1 counter on Vadrik."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("VadrikAstralArchmage.VadrikAstralArchmageEffect", CostKind::Reduce, applies, reduce)];

/// an instant or sorcery spell you cast (the face cast)
fn applies(_game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && (of.face.is(CardType::Instant) || of.face.is(CardType::Sorcery))
}

/// `CardUtil.reduceCost` by Vadrik's power (`SourcePermanentPowerValue.NOT_NEGATIVE`): off the generic mana
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let power = game.power_toughness(cx.source).0.max(0) as u32;
    cost.generic = cost.generic.saturating_sub(power);
}
