//! Fractalize: "Until end of turn, target creature becomes a green and blue Fractal with base power and toughness
//! each equal to X plus 1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Fractalize.FractalizeEffect", apply).effects(&[
    "new BecomesCreatureTargetEffect(new CreatureToken(0,0,\"green and blue Fractal\").withColor(\"GU\").withSubType(SubType.FRACTAL), false, false, Duration.EndOfTurn)",
])];

/// `BecomesCreatureTargetEffect` with the token's base power and toughness X plus 1, X the spell's (`init`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let x = cx.item.x.max(0) + 1;
    let mut e = cx.effects.first()?.clone();
    if let EffectDef::BecomesCreatureTargetEffect { token, .. } = &mut e {
        token.power = x;
        token.toughness = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}
