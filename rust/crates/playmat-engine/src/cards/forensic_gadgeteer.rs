//! Forensic Gadgeteer: "Whenever you cast an artifact spell, investigate. Activated abilities of artifacts you
//! control cost {1} less to activate. This effect can't reduce the mana in that cost to less than one mana."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "ForensicGadgeteer.ForensicGadgeteerEffect",
    CostKind::Reduce,
    artifact_you_control,
    up_to_one,
)];

/// an activated ability of an artifact you control (as it last was on the battlefield)
fn artifact_you_control(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_some()
        && game.obj(of.card).zone == Zone::Battlefield
        && game.obj(of.card).controller == cx.controller
        && game.has_type(of.card, CardType::Artifact)
}

/// `CardUtil.calculateActualPossibleGenericManaReduction(mana, 1, 1)`: one generic, leaving at least one mana
fn up_to_one(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let non_generic =
        cost.colored.iter().sum::<u32>() + cost.colorless + cost.hybrid.len() as u32 + cost.mono.len() as u32 + cost.snow;
    let keep = 1u32.saturating_sub(non_generic);
    let n = cost.generic.saturating_sub(keep).min(1);
    cost.generic -= n;
}
