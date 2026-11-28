//! The Scarlet Witch: "Instant and sorcery spells you cast with mana value 4 or greater cost {X} less to cast, where
//! X is The Scarlet Witch's power."

use super::*;
use crate::card::{CardType, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("TheScarletWitch.TheScarletWitchEffect", CostKind::Reduce, applies, reduce)];

fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && (of.face.is(CardType::Instant) || of.face.is(CardType::Sorcery))
        && of.face.mana_value() >= 4
}

/// `SourcePermanentPowerValue.NOT_NEGATIVE`
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let power = game.power_toughness(cx.source).0.max(0) as u32;
    cost.generic = cost.generic.saturating_sub(power);
}
