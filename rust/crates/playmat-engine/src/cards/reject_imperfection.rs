//! Reject Imperfection: "Counter target spell. If that spell's mana value was 3 or less, proliferate."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("RejectImperfection.RejectImperfectionEffect", apply)];

/// The target spell's mana value (X included, `Spell.getManaValue`) is read first; then it's countered
/// (`CounterTargetEffect`), and with that mana value 3 or less you proliferate, countered or not
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let cost = game.obj(spell).def.printed();
    let x = game.stack.iter().find(|i| i.card == spell && i.ability.is_none()).map_or(0, |i| i.x);
    let mv = cost.mana_value() as i32 + x * cost.x as i32;
    cx.run(game, d, &[EffectDef::CounterTargetEffect]);
    if d.stopped() {
        return None;
    }
    if mv <= 3 {
        cx.run(game, d, &[EffectDef::ProliferateEffect]);
    }
    Some(())
}
