//! Show of Confidence: "When you cast this spell, copy it for each other instant or sorcery spell you've cast this
//! turn. You may choose new targets for the copies."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("ShowOfConfidence.ShowOfConfidenceEffect", apply)];

/// The spell, still on the stack as the one cast (`getValue("spellCast")`), copied once for each instant and
/// sorcery spell you cast this turn but it (`InstantAndSorceryCastThisTurn.YOU - 1`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let (spell, zcc) = cx.source;
    let original =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()?;
    if game.obj(spell).zcc != zcc {
        return Some(());
    }
    let copies = game.player(you).instants_sorceries_cast as i32 - 1;
    for _ in 0..copies.max(0) {
        copy_spell(game, d, &original, you);
        if d.stopped() {
            return None;
        }
    }
    Some(())
}
