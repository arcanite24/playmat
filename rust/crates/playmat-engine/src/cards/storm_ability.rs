//! Storm (`StormAbility`): "When you cast this spell, copy it for each spell cast before it this turn. You may choose
//! new targets for the copies."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger("StormAbility", "new CastSourceTriggeredAbility(new StormEffect())", always),
    Handler::effect("StormEffect", apply),
];

fn always(_: &Game, _: &StackItem) -> bool {
    true
}

/// `CastSpellLastTurnWatcher.getSpellOrder(spell) - 1`: the spells anyone cast this turn before this one (its own
/// cast is the latest of this card's), then `spell.createCopyOnStack(…, true, stormCount)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let (spell, zcc) = cx.source;
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    if game.obj(spell).zcc != zcc {
        return Some(());
    }
    let Some(order) = game.watch.spells.iter().rposition(|s| s.card == spell) else { return Some(()) };
    for _ in 0..order {
        copy_spell(game, d, &original, cx.controller());
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
