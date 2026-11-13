//! Rakdos, Lord of Riots: "You can't cast this spell unless an opponent lost life this turn." and "Creature spells
//! you cast cost {1} less to cast for each 1 life your opponents have lost this turn."

use super::*;
use crate::card::{CardDef, CardType, ManaCost};

pub const HANDLERS: &[Handler] = &[
    Handler::rule("RakdosLordOfRiots.RakdosLordOfRiotsCantCastEffect", Rule::CantCast(cant_cast)).in_any_zone(),
    Handler::cost("RakdosLordOfRiots.RakdosLordOfRiotsCostReductionEffect", CostKind::Reduce, applies, reduce),
];

/// `OpponentsLostLifeCount`
fn opponents_lost(game: &Game, you: Seat) -> i32 {
    (0..game.seats()).filter(|p| *p != you).map(|p| game.player(p).life_lost).sum()
}

fn cant_cast(game: &Game, cx: &StaticCx, _: Seat, id: ObjId, _: &CardDef) -> bool {
    id == cx.source && opponents_lost(game, cx.controller) == 0
}

fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.seat == cx.controller && of.face.is(CardType::Creature)
}

fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(opponents_lost(game, cx.controller).max(0) as u32);
}
