//! `ManifestDreadThenAttachEffect`: "When this Equipment enters, manifest dread, then attach this Equipment to that
//! creature." (Conductive Machete, Cursed Windbreaker, Dissection Tools, Killer's Mask). `doManifestDread` as the
//! engine's `ManifestDreadEffect` does it, then the Equipment, if it's still on the battlefield, is attached to the
//! manifested creature (`Permanent.addAttachment`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ManifestDreadThenAttachEffect", apply)];

fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    // `doManifestDread`: one of the top two chosen (one alone is taken)
    let top: Vec<ObjId> = game.player(you).library.iter().take(2).copied().collect();
    let card = match top.len() {
        0 => return Some(()),
        1 => top[0],
        _ => match choose_cards(game, d, you, &top, &top, 1, 1, "to manifest") {
            Some(c) if !c.is_empty() => c[0],
            _ => return None,
        },
    };
    let made = manifest(game, d, you, &[card], false).first().copied();
    manifest_dread_rest(game, you, &top);
    if let Some(p) = made {
        if cx.source_on_battlefield(game) && game.obj(p).zone == Zone::Battlefield {
            let z = game.obj(p).zcc;
            game.attach_to(cx.source_id(), p, z);
        }
    }
    Some(())
}
