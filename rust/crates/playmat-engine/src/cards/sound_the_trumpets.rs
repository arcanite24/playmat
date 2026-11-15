//! Sound the Trumpets: "Counter target spell. If that spell's mana value was 2 or less, recruit."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("SoundTheTrumpets.SoundTheTrumpetsEffect", apply).tokens(&["HumanSoldierToken"])];

/// the spell's mana value (its X included) as it was on the stack, then the counter, then recruit
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
    if mv <= 2 {
        cx.run(game, d, &[EffectDef::RecruitEffect { token: cx.tokens[0].clone() }]);
    }
    (!d.stopped()).then_some(())
}
