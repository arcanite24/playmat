//! Spell Swindle: "Counter target spell. Create X Treasure tokens, where X is that spell's mana value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("SpellSwindle.SpellSwindleEffect", apply)
    .effects(&["new CounterTargetEffect()", "new CreateTokenEffect(new TreasureToken())"])];

/// the spell's mana value (its X included) as it was on the stack, then the counter, then the Treasures
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
    if mv > 0 {
        let mut e = cx.effects[1].clone();
        if let EffectDef::CreateTokenEffect { amount, .. } = &mut e {
            *amount = Amount::Fixed(mv);
        }
        cx.run_on(game, d, &[e], Vec::new());
    }
    Some(())
}
