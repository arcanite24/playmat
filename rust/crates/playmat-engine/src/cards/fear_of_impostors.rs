//! Fear of Impostors: "When Fear of Impostors enters, counter target spell. Its controller manifests dread."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("FearOfImpostors.FearOfImpostorsEffect", apply)];

/// The spell's controller (read before it's countered) manifests dread after the counter, whether or not it was
/// countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    let p = game.obj(spell).controller;
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if !game.player(p).lost {
        cx.run_as(game, d, &[EffectDef::ManifestDreadEffect { counters: Vec::new() }], p);
    }
    Some(())
}
