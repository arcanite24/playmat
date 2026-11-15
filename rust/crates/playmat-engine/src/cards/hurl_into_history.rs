//! Hurl into History: "Counter target artifact or creature spell. Discover X, where X is that spell's mana value."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("HurlIntoHistory.HurlIntoHistoryEffect", apply).effects(&["new CounterTargetEffect()"])];

/// the spell's mana value (its X included) as it was on the stack, then the counter, then discover
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let Some(x) = game.stack.iter().rev().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.x) else {
        return Some(());
    };
    let mv = {
        let c = game.obj(spell).def.printed();
        c.mana_value() as i32 + x * c.x as i32
    };
    cx.run(game, d, &cx.effects[..1]);
    if d.stopped() {
        return None;
    }
    if !discover(game, d, cx.controller(), mv) {
        return None;
    }
    Some(())
}
