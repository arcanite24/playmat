//! Smash to Smithereens: "Destroy target artifact. Smash to Smithereens deals 3 damage to that artifact's
//! controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SmashToSmithereens.SmashToSmithereensEffect", apply)];

/// The artifact's controller as it was before it's destroyed, then the damage from the spell
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let controller = game.obj(o).controller;
    destroy(game, o);
    deal_damage(game, cx.source_id(), Target::Player(controller), 3, false);
    Some(())
}
