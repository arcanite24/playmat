//! Mask of the Jadecrafter: "{X}, {T}, Sacrifice Mask of the Jadecrafter: Create an X/X colorless Golem artifact
//! creature token. Activate only as a sorcery."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MaskOfTheJadecrafter.MaskOfTheJadecrafterEffect", apply).tokens(&["GolemXXToken"])];

/// `new GolemXXToken(X).putOntoBattlefield(1, …)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(cx.item.x) };
    cx.run_on(game, d, &[e], Vec::new());
    (!d.stopped()).then_some(())
}
