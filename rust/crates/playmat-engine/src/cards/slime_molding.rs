//! Slime Molding: "Create an X/X green Ooze creature token."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SlimeMolding.SlimeMoldingEffect", apply).tokens(&["OozeToken"])];

/// `new OozeToken()` with X power and toughness
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: crate::card::Amount::Fixed(cx.item.x) };
    cx.run(game, d, &[e]);
    (!d.stopped()).then_some(())
}
