//! The Lord of the Eagles: "Flash. This spell costs {X} less to cast, where X is the total power of creatures you
//! control with flying. Flying."

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "TheLordOfTheEagles.TheLordOfTheEaglesCostReductionEffect",
    CostKind::Reduce,
    own_spell,
    flying_power,
)
.in_any_zone()];

/// its own spell
fn own_spell(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.card == cx.source
}

/// `CardUtil.reduceCost(ability, total power of your creatures with flying)`: that much generic
fn flying_power(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let total: i32 = game
        .battlefield
        .iter()
        .filter(|b| {
            game.obj(**b).controller == cx.controller
                && game.has_type(**b, CardType::Creature)
                && game.has_ability(**b, &AbilityDef::FlyingAbility)
        })
        .map(|b| game.power_toughness(*b).0)
        .sum();
    cost.generic = cost.generic.saturating_sub(total.max(0) as u32);
}
