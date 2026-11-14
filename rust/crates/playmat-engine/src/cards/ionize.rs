//! Ionize: "Counter target spell. Ionize deals 2 damage to that spell's controller."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Ionize.IonizeEffect", apply)];

/// the spell's controller (read before the counter) is dealt 2 damage whether or not it was countered
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    let p = game.obj(spell).controller;
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if d.stopped() {
        return None;
    }
    if !game.player(p).lost {
        deal_damage(game, cx.source_id(), Target::Player(p), 2, false);
    }
    Some(())
}
