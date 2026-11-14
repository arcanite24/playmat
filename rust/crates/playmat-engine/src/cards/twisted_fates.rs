//! Twisted Fates: "Destroy target nonland permanent. Put a +1/+1 counter on each creature target player controls."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TwistedFates.TwistedFatesEffect", apply)];

/// the second target (tag 2), a player: a +1/+1 counter on each creature they control
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(p) = cx.player(1) else { return Some(()) };
    let mine: Vec<ObjId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|o| game.obj(*o).controller == p && game.has_type(*o, CardType::Creature))
        .collect();
    for o in mine {
        put_counters(game, o, "+1/+1", 1, cx.controller());
    }
    Some(())
}
