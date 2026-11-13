//! Didn't Say Please: "Counter target spell. Its controller mills three cards."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("DidntSayPlease.DidntSayPleaseEffect", apply).effects(&["new CounterTargetEffect()"])];

/// Java mills first, then counters
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let Some(p) = game.stack.iter().rev().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.controller) else {
        return Some(());
    };
    let top: Vec<ObjId> = game.player(p).library.iter().take(3).copied().collect();
    note(game, EventKind::Milled, Some(p), None, None, top.len() as i32);
    for c in top {
        game.move_to(c, Zone::Graveyard);
    }
    cx.run(game, d, &cx.effects[..1]);
    Some(())
}
