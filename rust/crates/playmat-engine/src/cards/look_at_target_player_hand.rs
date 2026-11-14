//! `LookAtTargetPlayerHandEffect` (Dragon's Eye Savants…): "look at target opponent's hand", titled by the source
//! (`lookAtCards`).

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("LookAtTargetPlayerHandEffect", apply)];

fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let p = cx.player(0)?;
    let cards = game.player(p).hand.clone();
    if !cards.is_empty() {
        let (source, zcc) = cx.source;
        game.looked_at.push(crate::state::LookedAt { viewer: cx.controller(), source, zcc, bare: false, cards });
    }
    Some(())
}
