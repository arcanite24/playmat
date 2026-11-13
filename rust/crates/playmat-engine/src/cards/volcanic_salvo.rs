//! Volcanic Salvo: "This spell costs {X} less to cast, where X is the total power of creatures you control."

use super::*;
use crate::card::{CardType, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "VolcanicSalvo.VolcanicSalvoCostReductionEffect",
    CostKind::Reduce,
    applies,
    reduce,
)
.in_any_zone()];

fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.card == cx.source
}

fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let power: i32 = game
        .battlefield
        .iter()
        .filter(|c| game.obj(**c).controller == cx.controller && game.has_type(**c, CardType::Creature))
        .map(|c| game.power_toughness(*c).0)
        .sum();
    cost.generic = cost.generic.saturating_sub(power.max(0) as u32);
}
