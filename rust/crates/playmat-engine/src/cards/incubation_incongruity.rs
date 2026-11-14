//! Incubation // Incongruity: "Exile target creature. That creature's controller creates a 3/3
//! green Frog Lizard creature token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("IncubationIncongruity.IncongruityEffect", apply).tokens(&["FrogLizardToken"])];

/// `getFirstTargetPermanentOrLKI`: the exiled creature's controller as it last was
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateTokenControllerTargetEffect { token: cx.tokens[0].clone(), amount: 1, tapped: false };
    cx.run(game, d, &[e]);
    Some(())
}
