//! Rapid Hybridization: "Destroy target creature. It can't be regenerated. That creature's controller creates a 3/3
//! green Frog Lizard creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("RapidHybridization.RapidHybridizationEffect", apply).tokens(&["FrogLizardToken"])];

/// `getFirstTargetPermanentOrLKI`: the destroyed creature's controller as it last was
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenControllerTargetEffect { token: cx.tokens[0].clone(), amount: 1, tapped: false };
    cx.run(game, d, &[e]);
    Some(())
}
