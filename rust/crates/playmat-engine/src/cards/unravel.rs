//! Unravel: "Counter target spell. If the amount of mana spent to cast that spell was less than its mana value, you
//! draw a card."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("Unravel.UnravelEffect", apply)];

/// the mana spent on it against its mana value (its X included) on the stack, then the counter, then the draw
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    let x = game.stack.iter().rev().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.x)?;
    let flag = {
        let c = game.obj(spell).def.printed();
        let mv = c.mana_value() as i32 + x * c.x as i32;
        (game.obj(spell).mana_spent as i32) < mv
    };
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if d.stopped() {
        return None;
    }
    if flag {
        draw(game, cx.controller(), 1);
    }
    Some(())
}
