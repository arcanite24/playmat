//! Smoke Teller: "{1}{U}: Look at target face-down creature."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SmokeTeller.SmokeTellerLookFaceDownEffect", apply)];

/// `lookAtCards` of the target permanent, by the ability's controller (only they see it)
fn apply(game: &mut Game, _: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let t = cx.obj(0)?;
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let zcc = game.obj(cx.source_id()).zcc;
    game.looked_at.push(crate::state::LookedAt { viewer: you, source: cx.source_id(), zcc, bare: false, cards: vec![t] });
    Some(())
}
