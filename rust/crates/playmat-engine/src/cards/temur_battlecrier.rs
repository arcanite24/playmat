//! Temur Battlecrier: "During your turn, spells you cast cost {1} less to cast for each creature you control with
//! power 4 or greater."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("TemurBattlecrier.TemurBattlecrierEffect", CostKind::Reduce, applies, reduce)];

/// a spell you cast during your turn
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller && game.active == Some(cx.controller)
}

/// {1} less for each creature you control with power 4 or greater (`CardUtil.reduceCost`: generic only)
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n = game
        .battlefield
        .iter()
        .filter(|c| {
            game.obj(**c).controller == cx.controller
                && game.has_type(**c, CardType::Creature)
                && game.power_toughness(**c).0 > 3
        })
        .count() as u32;
    cost.generic = cost.generic.saturating_sub(n);
}
