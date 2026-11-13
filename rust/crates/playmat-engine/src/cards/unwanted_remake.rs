//! Unwanted Remake: "Destroy target creature. Its controller manifests dread."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnwantedRemake.UnwantedRemakeEffect", apply)];

/// Its controller manifests dread whether or not it was destroyed
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let p = game.obj(o).controller;
    cx.run_on(game, d, &[EffectDef::DestroyTargetEffect], vec![Target::Obj(o)]);
    cx.run_as(game, d, &[EffectDef::ManifestDreadEffect { counters: Vec::new() }], p);
    Some(())
}
