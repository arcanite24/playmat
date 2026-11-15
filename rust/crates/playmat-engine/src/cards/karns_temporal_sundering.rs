//! Karn's Temporal Sundering: "(You may cast a legendary sorcery only if you control a legendary creature or
//! planeswalker.) Target player takes an extra turn after this one. Return up to one target nonland permanent to its
//! owner's hand. Exile Karn's Temporal Sundering."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KarnsTemporalSundering.KarnsTemporalSunderingEffect", apply)];

/// The first target (a player) takes an extra turn after this one (`TurnMod.withExtraTurn`), then the second
/// target's permanent goes to its owner's hand
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    if let Some(p) = cx.player(0) {
        game.extra_turns.push(p);
    }
    if let Some(o) = cx.obj(1).filter(|o| game.obj(*o).zone == Zone::Battlefield) {
        cx.run_on(game, d, &[EffectDef::ReturnToHandTargetEffect], vec![Target::Obj(o)]);
    }
    if d.stopped() {
        return None;
    }
    Some(())
}
