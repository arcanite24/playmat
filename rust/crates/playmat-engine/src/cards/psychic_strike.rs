//! Psychic Strike: "Counter target spell. Its controller mills two cards."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("PsychicStrike.PsychicStrikeEffect", apply)
    .effects(&["new CounterTargetEffect()", "new MillCardsControllerEffect(2)"])];

/// the spell's controller, read before it's countered, mills
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    let owner = game.obj(o).controller;
    cx.run(game, d, &cx.effects[..1]);
    cx.run_as(game, d, &cx.effects[1..2], owner);
    if d.stopped() {
        return None;
    }
    Some(())
}
