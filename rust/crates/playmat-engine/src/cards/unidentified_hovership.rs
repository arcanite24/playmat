//! Unidentified Hovership: "When this Vehicle leaves the battlefield, the exiled card's owner manifests dread."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("UnidentifiedHovership.UnidentifiedHovershipEffect", apply)];

/// the cards its enters trigger exiled (`getExileZoneId(…, -1)`: the permanent it was), still in exile
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (src, zcc) = cx.source;
    let exiled: Vec<ObjId> = game
        .exiled_for_source
        .iter()
        .filter(|(s, z, c)| *s == src && *z + 1 == zcc && game.obj(*c).zone == Zone::Exile)
        .map(|(_, _, c)| *c)
        .collect();
    for c in exiled {
        let owner = game.obj(c).owner;
        cx.run_as(game, d, &[EffectDef::ManifestDreadEffect { counters: Vec::new() }], owner);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
