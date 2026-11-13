//! Brush Off: "This spell costs {1}{U} less to cast if it targets an instant or sorcery spell." (on the stack only,
//! `setCanWorksOnStackOnly`: while checking what's playable it counts as cheaper)

use super::*;
use crate::card::{CardType, ManaCost};
use crate::state::Zone;

pub const HANDLERS: &[Handler] = &[Handler::cost(
    "BrushOff.SpellCostReductionSourceEffect",
    CostKind::Reduce,
    applies,
    reduce,
)
.args(&["one_of:new ManaCostsImpl<>(\"{1}{U}\")", "one_of:BrushOffCondition.instance"])
.in_any_zone()];

fn applies(game: &Game, cx: &StaticCx, of: &CostCx) -> bool {
    if of.ability.is_some() || of.card != cx.source {
        return false;
    }
    if game.obj(of.card).zone != Zone::Stack {
        return true;
    }
    // `BrushOffCondition`: one of its targets is an instant or sorcery spell
    game.stack.iter().rev().find(|t| t.card == of.card && t.ability.is_none()).is_some_and(|t| {
        t.targets.iter().flatten().any(|x| match x {
            Target::Obj(s) => {
                let o = game.obj(*s);
                o.zone == Zone::Stack && (o.def.is(CardType::Instant) || o.def.is(CardType::Sorcery))
            }
            _ => false,
        })
    })
}

/// `CardUtil.reduceCost(ability, {1}{U})`: a blue symbol and one generic
fn reduce(_: &Game, _: &StaticCx, _: &CostCx, cost: &mut ManaCost) {
    cost.colored[1] = cost.colored[1].saturating_sub(1);
    cost.generic = cost.generic.saturating_sub(1);
}
