//! Everflame Eidolon: "Bestow {2}{R}. {R}: Everflame Eidolon gets +1/+0 until end of turn. If it's an Aura, enchanted
//! creature gets +1/+0 until end of turn instead. Enchanted creature gets +1/+1."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("EverflameEidolon.EverflameEidolonEffect", apply)];

/// An Aura: the creature it enchants now; else the Eidolon itself
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let src = cx.source_id();
    if game.has_subtype(src, "Aura") {
        let Some((to, zcc)) = game.obj(src).attached_to else { return Some(()) };
        if game.obj(src).zone == Zone::Battlefield && game.obj(to).zone == Zone::Battlefield && game.obj(to).zcc == zcc {
            boost(game, to, 1, 0, crate::card::Duration::EndOfTurn);
        }
    } else if cx.source_on_battlefield(game) {
        boost(game, src, 1, 0, crate::card::Duration::EndOfTurn);
    }
    Some(())
}
