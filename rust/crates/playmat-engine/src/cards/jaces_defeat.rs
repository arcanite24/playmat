//! Jace's Defeat: "Counter target blue spell. If it was a Jace planeswalker spell, scry 2."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("JacesDefeat.JacesDefeatEffect", apply).effects(&["new CounterTargetEffect()"])];

/// The counter, then scry 2 when it was a Jace planeswalker spell
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let o = cx.obj(0)?;
    if game.obj(o).zone != Zone::Stack {
        return None;
    }
    let jace = game.has_type(o, CardType::Planeswalker) && game.has_subtype(o, "Jace");
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if jace {
        let rule = item_rule(game, cx.item);
        if !scry(game, d, cx.controller(), 2, &rule) {
            return None;
        }
    }
    Some(())
}
