//! Chandra's Incinerator: "This spell costs {X} less to cast, where X is the total amount of noncombat damage dealt
//! to your opponents this turn."

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "ChandrasIncinerator.ChandrasIncineratorCostReductionEffect",
    CostKind::Reduce,
    applies,
    reduce,
)
.in_any_zone()];

/// its own spell
fn applies(_: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    of.ability.is_none() && of.card == cx.source
}

/// `ChandrasIncineratorWatcher`: each noncombat `DAMAGED_PLAYER` this turn counts for the damaged player's opponents
fn reduce(game: &Game, cx: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    let n: i32 = game
        .watch
        .damage
        .iter()
        .filter(|d| !d.combat && matches!(d.to, Target::Player(p) if cx.opponent(p)))
        .map(|d| d.amount)
        .sum();
    cost.generic = cost.generic.saturating_sub(n.max(0) as u32);
}
