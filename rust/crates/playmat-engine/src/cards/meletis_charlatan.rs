//! Meletis Charlatan: "{2}{U}, {T}: The controller of target instant or sorcery spell copies it. That player may
//! choose new targets for the copy."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("MeletisCharlatan.MeletisCharlatanCopyTargetSpellEffect", apply)];

/// `spell.createCopyOnStack(game, source, spell.getControllerId(), true)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.obj(0) else { return Some(()) };
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    let owner = original.controller;
    copy_spell_with(game, d, &original, owner, true, false)?;
    Some(())
}
