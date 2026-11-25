//! Teferi, Who Slows the Sunset: "+1: Choose up to one target artifact, up to one target creature, and up to one
//! target land. Untap the chosen permanents you control. Tap the chosen permanents you don't control. You gain 2
//! life." Its emblem's "You draw a card during each opponent's draw step." is `DrawsOnOpponentsTurnEffect`.

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TeferiWhoSlowsTheSunset.TeferiWhoSlowsTheSunsetEffect", apply)];

/// each target group's first permanent still there, in target order: untapped if you control it, else tapped
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    for i in 0..cx.legal.len() {
        let Some(o) = cx.obj(i) else { continue };
        if game.obj(o).zone != Zone::Battlefield {
            continue;
        }
        let e = if game.obj(o).controller == you { EffectDef::UntapTargetEffect } else { EffectDef::TapTargetEffect };
        cx.run_on(game, d, &[e], vec![Target::Obj(o)]);
    }
    Some(())
}
