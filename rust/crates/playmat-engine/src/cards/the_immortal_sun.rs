//! The Immortal Sun: "Players can't activate planeswalkers' loyalty abilities. At the beginning of your draw step,
//! draw an additional card. Spells you cast cost {1} less to cast. Creatures you control get +1/+1."

use super::*;
use crate::card::AbilityDef;

pub const HANDLERS: &[Handler] =
    &[Handler::rule("TheImmortalSun.TheImmortalSunCantActivateEffect", Rule::CantActivate(loyalty))];

/// `ACTIVATE_ABILITY` of a planeswalker's `LoyaltyAbility`
fn loyalty(game: &Game, _: &StaticCx, _: Seat, id: ObjId, index: usize) -> bool {
    game.has_type(id, CardType::Planeswalker)
        && matches!(game.obj(id).def.abilities.get(index), Some(AbilityDef::LoyaltyAbility { .. }))
}
