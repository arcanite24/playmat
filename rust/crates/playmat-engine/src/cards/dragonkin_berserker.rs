//! Dragonkin Berserker: "First strike. Boast abilities you activate cost {1} less to activate for each Dragon you
//! control. Boast — {4}{R}: Create a 5/5 red Dragon creature token with flying."

use super::*;
use crate::card::{Condition, ManaCost};

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "DragonkinBerserker.DragonkinBerserkerEffect",
    CostKind::Reduce,
    your_boast,
    per_dragon,
)];

/// a boast ability (`instanceof BoastAbility`: once each turn, only if its source attacked this turn) you activate
fn your_boast(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.seat == cx.controller
        && matches!(
            of.ability,
            Some(AbilityDef::LimitedTimesPerTurnActivatedAbility {
                limit: 1,
                condition: Some(Condition::SourceAttackedThisTurn),
                ..
            })
        )
}

/// `CardUtil.reduceCost(ability, n)`: {1} less for each Dragon you control
fn per_dragon(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n = game
        .battlefield
        .iter()
        .filter(|o| game.obj(**o).controller == cx.controller && game.has_subtype(**o, "Dragon"))
        .count() as u32;
    cost.generic = cost.generic.saturating_sub(n);
}
