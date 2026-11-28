//! Reality Shift: "Exile target creature. Its controller manifests the top card of their library."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RealityShift.RealityShiftEffect", apply)];

/// `ManifestTargetPlayerEffect(1)` for the exiled creature's controller (as the engine last knows it: once exiled,
/// its owner, which is its controller unless it was stolen)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let p = game.obj(o).controller;
    let top: Vec<ObjId> = game.player(p).library.iter().take(1).copied().collect();
    manifest(game, d, p, &top, false);
    if d.stopped() {
        return None;
    }
    Some(())
}
