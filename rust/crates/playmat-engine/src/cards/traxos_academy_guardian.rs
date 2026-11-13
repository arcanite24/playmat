//! Traxos, Academy Guardian: "This spell costs {2} less to cast if you've cast a noncreature spell this turn."
//! (`SpellCostReductionSourceEffect({2}, StormwingEntityCondition)`, in any zone: its own spell)

use super::*;
use crate::card::ManaCost;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "TraxosAcademyGuardian.SpellCostReductionSourceEffect",
    CostKind::Reduce,
    applies,
    reduce,
)
.args(&["one_of:new ManaCostsImpl<>(\"{2}\")", "one_of:StormwingEntityCondition.instance"])
.in_any_zone()];

fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    // `SpellsCastWatcher`: a noncreature spell of the controller's this turn (other than this one, a creature)
    of.ability.is_none() && of.card == cx.source && game.player(cx.controller).noncreature_cast > 0
}

fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.generic = cost.generic.saturating_sub(2);
}
