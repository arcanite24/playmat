//! Summary Judgment: "Summary Judgment deals 3 damage to target tapped creature. Addendum — If you cast this spell
//! during your main phase, it deals 5 damage instead."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SummaryJudgment.SummaryJudgementEffect", apply)];

/// `AddendumCondition`: your main phase, this a spell on the stack
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let you = cx.controller();
    let addendum = game.active == Some(you)
        && game.step.is_some_and(|s| s.is_main())
        && game.obj(cx.source_id()).zone == Zone::Stack;
    deal_damage(game, cx.source_id(), Target::Obj(o), if addendum { 5 } else { 3 }, false);
    Some(())
}
