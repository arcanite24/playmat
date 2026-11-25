//! Oppressive Rays: "Activated abilities of enchanted creature cost {3} more to activate." (The attack and block tax
//! is `CantAttackBlockUnlessPaysSourceEffect` with `attached`.)

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "OppressiveRays.OppressiveRaysCostModificationEffect",
    CostKind::Increase,
    enchanted_creatures_ability,
    three_more,
)];

/// an activated ability whose object is the creature this Aura is attached to
fn enchanted_creatures_ability(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_some() && game.attached(cx.source) == Some(of.card)
}

fn three_more(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic += 3;
}
