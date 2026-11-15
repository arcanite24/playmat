//! Anointed Peacekeeper: "Vigilance. As Anointed Peacekeeper enters, look at an opponent's hand, then choose any card
//! name. Spells your opponents cast with the chosen name cost {2} more to cast. Activated abilities of sources with
//! the chosen name cost {2} more to activate unless they're mana abilities."

use super::*;
use crate::card::{AbilityDef, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "AnointedPeacekeeper.AnointedPeacekeeperEffect",
    CostKind::Increase,
    named_source_ability,
    two_more,
)];

/// `isNonManaActivatedAbility` (a Room's unlock is a special action) of a source with the chosen name
/// (`CardUtil.haveSameNames`), anyone's
fn named_source_ability(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    let Some(a) = of.ability else { return false };
    if matches!(a, AbilityDef::RoomUnlockAbility { .. }) {
        return false;
    }
    game.obj(cx.source).chosen_name.as_ref().is_some_and(|n| !n.is_empty() && *n == game.obj(of.card).name())
}

/// `CardUtil.increaseCost(ability, 2)`
fn two_more(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 2;
}
