//! Vraska, the Cutting Glare: "Deathtouch. When Vraska enters, if you control six or more lands, destroy target
//! permanent an opponent controls. They create a Treasure token."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("VraskaTheCuttingGlare.VraskaTheCuttingGlareEffect", apply).tokens(&["TreasureToken"])];

/// the target destroyed (whether or not it is), then a Treasure for its controller
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(target) = cx.obj(0) else { return Some(()) };
    let controller = game.obj(target).controller;
    destroy(game, target);
    let token = token_effect(&cx.tokens[0], 1);
    cx.run_as(game, d, &[token], controller);
    Some(())
}
