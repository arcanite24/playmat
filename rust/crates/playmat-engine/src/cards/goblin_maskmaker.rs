//! Goblin Maskmaker: "Whenever Goblin Maskmaker attacks, face-down spells you cast this turn cost {1} less to cast."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("GoblinMaskmaker.GoblinMaskmakerEffect", CostKind::Reduce, face_down_spell, one_less)];

/// until the turn's cleanup: spells you cast face down (`getSpellAbilityCastMode().isFaceDown()`: morph or disguise,
/// the face cast being the nameless face-down 2/2)
fn face_down_spell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && of.face.name.is_empty()
        && of.face.abilities.iter().any(|a| matches!(a, AbilityDef::TurnFaceUpAbility { .. }))
}

/// `CardUtil.reduceCost(ability, 1)`: one generic
fn one_less(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(1);
}
