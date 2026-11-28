//! Banefire: "Banefire deals X damage to any target. If X is 5 or more, this spell can't be countered and the
//! damage can't be prevented."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::effect("Banefire.BaneFireEffect", apply),
    Handler::condition("Banefire.testCondition", x_at_least_five),
];

/// X damage (the X paid, `getSourceCostsTag`) to the target, which can't be prevented when X is 5 or more
/// (`damage(…, preventable)`): no prevention for that one damage event
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let to = cx.target(0)?;
    let x = cx.item.x;
    if x >= 5 {
        let was = game.no_prevention_turn;
        game.no_prevention_turn = Some(game.turn);
        deal_damage(game, cx.source_id(), to, x, false);
        game.no_prevention_turn = was;
    } else {
        deal_damage(game, cx.source_id(), to, x, false);
    }
    Some(())
}

/// The spell on the stack has X of 5 or more (`testCondition(GetXValue.instance, 5)`, read from the spell)
fn x_at_least_five(game: &Game, _you: Seat, spell: ObjId) -> bool {
    game.stack.iter().any(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none() && i.x >= 5)
}
