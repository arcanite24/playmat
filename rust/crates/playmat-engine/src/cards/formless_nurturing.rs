//! Formless Nurturing: "Manifest the top card of your library, then put a +1/+1 counter on it."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("FormlessNurturing.FormlessNurturingEffect", apply).effects(&["new ManifestEffect(1)"])];

/// the library's top card, once it's manifested
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(top) = game.player(cx.controller()).library.first().copied() else { return Some(()) };
    cx.run_on(game, d, cx.effects, Vec::new());
    if d.stopped() {
        return None;
    }
    if game.obj(top).zone == Zone::Battlefield {
        put_counters(game, top, "+1/+1", 1, cx.controller());
    }
    Some(())
}
