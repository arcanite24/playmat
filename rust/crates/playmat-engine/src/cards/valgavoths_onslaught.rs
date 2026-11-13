//! Valgavoth's Onslaught: "Manifest dread X times, then put X +1/+1 counters on each of those creatures."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ValgavothsOnslaught.ValgavothsOnslaughtEffect", apply)];

/// X manifests dread one after the other (each a `doManifestDread`), then X +1/+1 counters on each permanent they
/// made that's still on the battlefield
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let x = cx.item.x;
    if x < 1 {
        return Some(());
    }
    let mut made: Vec<(ObjId, u32)> = Vec::new();
    for _ in 0..x {
        let before: Vec<ObjId> = game.battlefield.iter().copied().collect();
        cx.run(game, d, &[EffectDef::ManifestDreadEffect { counters: Vec::new() }]);
        if d.stopped() {
            return None;
        }
        for o in game.battlefield.iter().copied() {
            if !before.contains(&o) && game.obj(o).controller == you {
                made.push((o, game.obj(o).zcc));
            }
        }
    }
    for (o, zcc) in made {
        if game.obj(o).zone == Zone::Battlefield && game.obj(o).zcc == zcc {
            put_counters(game, o, "+1/+1", x, you);
        }
    }
    Some(())
}
