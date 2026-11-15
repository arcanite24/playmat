//! Rowan, Scion of War: "Menace. {T}: Spells you cast this turn that are black and/or red cost {X} less to cast,
//! where X is the amount of life you lost this turn. Activate only as a sorcery."

use super::*;
use crate::card::{Color, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("RowanScionOfWar.RowanScionOfWarEffect", CostKind::Reduce, applies, reduce)];

/// a spell you cast (the face cast) that's black or red
fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller && of.face.colors.iter().any(|c| matches!(c, Color::B | Color::R))
}

/// `CardUtil.reduceCost(ability, ControllerLostLifeCount)`: that much less generic mana
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(game.player(cx.controller).life_lost.max(0) as u32);
}
