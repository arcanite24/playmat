//! Bring the Ending: "Counter target spell unless its controller pays {2}. Corrupted — Counter that spell instead if
//! its controller has three or more poison counters."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("BringTheEnding.BringTheEndingCounterEffect", apply)
    .effects(&["new CounterTargetEffect()", "new CounterUnlessPaysEffect(new GenericManaCost(2))"])];

/// the spell's controller's poison counters pick `CounterTargetEffect` (three or more) or `CounterUnlessPaysEffect`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let controller = game
        .stack
        .iter()
        .find(|i| i.card == spell && i.ability.is_none())
        .map_or(game.obj(spell).controller, |i| i.controller);
    let poison: i32 =
        game.player(controller).counters.iter().filter(|(c, _)| c == "poison").map(|(_, n)| *n).sum();
    let e = if poison >= 3 { &cx.effects[0] } else { &cx.effects[1] };
    cx.run(game, d, std::slice::from_ref(e));
    Some(())
}
