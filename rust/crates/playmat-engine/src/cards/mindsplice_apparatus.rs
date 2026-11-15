//! Mindsplice Apparatus: "Instant and sorcery spells you cast cost {1} less to cast for each oil counter on
//! Mindsplice Apparatus."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] =
    &[Handler::cost("MindspliceApparatus.MindspliceApparatusEffect", CostKind::Reduce, applies, reduce)];

/// instant and sorcery spells you cast
fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none()
        && of.seat == cx.controller
        && (game.has_type(of.card, CardType::Instant) || game.has_type(of.card, CardType::Sorcery))
}

/// {1} less for each oil counter on it
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n = game.obj(cx.source).counter("oil").max(0) as u32;
    cost.generic = cost.generic.saturating_sub(n);
}
