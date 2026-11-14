//! Precognitive Perception: "Draw three cards. Addendum — If you cast this spell during your main phase, instead
//! scry 3, then draw three cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PrecognitivePerception.PrecognitivePerceptionEffect", apply)];

/// `AddendumCondition` as it resolves (your turn, a main phase, the spell on the stack), then the draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let addendum = game.active == Some(you)
        && game.step.is_some_and(|s| s.is_main())
        && game.obj(cx.source_id()).zone == Zone::Stack;
    if addendum {
        cx.run(game, d, &[EffectDef::ScryEffect { amount: Amount::Fixed(3) }]);
        if d.stopped() {
            return None;
        }
    }
    draw(game, you, 3);
    Some(())
}
