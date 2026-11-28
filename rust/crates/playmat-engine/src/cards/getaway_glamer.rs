//! Getaway Glamer's second mode: "Destroy target creature if no other creature has greater power."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GetawayGlamer.GetawayGlamerEffect", apply)];

/// the mode's own target (each mode resolves with its targets); any creature with greater power stops it
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(t) = cx.obj(0) else { return Some(()) };
    if game.obj(t).zone != Zone::Battlefield {
        return Some(());
    }
    let p = game.power_toughness(t).0;
    let greater = game
        .battlefield
        .iter()
        .any(|o| game.has_type(*o, CardType::Creature) && game.power_toughness(*o).0 > p);
    if !greater {
        destroy(game, t);
    }
    Some(())
}
