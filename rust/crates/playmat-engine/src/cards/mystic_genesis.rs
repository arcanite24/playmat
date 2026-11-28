//! Mystic Genesis: "Counter target spell. Create an X/X green Ooze creature token, where X is that spell's mana
//! value."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("MysticGenesis.MysticGenesisEffect", apply).tokens(&["OozeToken"])];

/// the spell's mana value (its X included) as it was on the stack, then the counter, then the Ooze
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let spell = cx.obj(0)?;
    let x = game.stack.iter().rev().find(|i| i.card == spell && i.ability.is_none()).map(|i| i.x)?;
    let mv = {
        let c = game.obj(spell).def.printed();
        c.mana_value() as i32 + x * c.x as i32
    };
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if d.stopped() {
        return None;
    }
    let e = EffectDef::CreateXXTokenEffect { token: cx.tokens[0].clone(), amount: Amount::Fixed(mv) };
    cx.run_on(game, d, &[e], Vec::new());
    (!d.stopped()).then_some(())
}
