//! Agatha of the Vile Cauldron: "Activated abilities of creatures you control cost {X} less to activate, where X is
//! Agatha of the Vile Cauldron's power. This effect can't reduce the mana in that cost to less than one mana."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("AgathaOfTheVileCauldron.AgathaOfTheVileCauldronEffect", CostKind::Reduce, applies, reduce)];

/// an activated ability you activate of a creature on the battlefield
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_some()
        && of.seat == cx.controller
        && game.obj(of.card).zone == Zone::Battlefield
        && game.has_type(of.card, CardType::Creature)
}

/// Agatha's power (not below 0) off the generic mana, leaving at least one mana in the cost
/// (`CardUtil.calculateActualPossibleGenericManaReduction(mana, power, 1)`)
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let power = game.power_toughness(cx.source).0.max(0) as u32;
    if power == 0 {
        return;
    }
    let non_generic = cost.mana_value().saturating_sub(cost.generic);
    let keep = 1u32.saturating_sub(non_generic);
    let can = cost.generic.saturating_sub(keep).min(power);
    cost.generic -= can;
}
