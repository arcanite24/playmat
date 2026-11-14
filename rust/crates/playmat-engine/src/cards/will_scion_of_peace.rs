//! Will, Scion of Peace: "{T}: Spells you cast this turn that are white and/or blue cost {X} less to cast, where X is
//! the amount of life you gained this turn. Activate only as a sorcery." (resolved: until the turn's cleanup)

use super::*;
use crate::card::{Color, ManaCost};

pub const HANDLERS: &[Handler] =
    &[Handler::cost("WillScionOfPeace.WillScionOfPeaceEffect", CostKind::Reduce, applies, reduce)];

/// your spells that are white or blue
fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller && of.face.colors.iter().any(|c| matches!(c, Color::W | Color::U))
}

/// `CardUtil.reduceCost(ability, X)`: X generic less, X the life you gained this turn (`ControllerGainedLifeCount`)
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let x = game.player(cx.controller).life_gained.max(0) as u32;
    cost.generic = cost.generic.saturating_sub(x);
}
