//! Countermand: "Counter target spell. Its controller mills four cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Countermand.CountermandEffect", apply)];

/// Java reads the spell's controller first, counters it, then that player mills four (even if it couldn't be countered)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let owner = game.obj(spell).controller;
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    let mill = EffectDef::MillCardsTargetEffect { amount: crate::card::Amount::Fixed(4) };
    cx.run_on(game, d, &[mill], vec![Target::Player(owner)]);
    Some(())
}
