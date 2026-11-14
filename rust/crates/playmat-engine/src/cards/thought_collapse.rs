//! Thought Collapse: "Counter target spell. Its controller mills three cards."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("ThoughtCollapse.ThoughtCollapseEffect", apply).effects(&["new CounterTargetEffect()"])];

/// The spell's controller mills three, then the spell is countered (Java's order)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let s = cx.obj(0)?;
    let p = game.obj(s).controller;
    let e = EffectDef::MillCardsControllerEffect { amount: crate::card::Amount::Fixed(3) };
    cx.run_as(game, d, &[e], p);
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
