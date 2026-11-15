//! Jin-Gitaxias, Progress Tyrant: "Whenever you cast an artifact, instant, or sorcery spell, copy that spell. You may
//! choose new targets for the copy. This ability triggers only once each turn. Whenever an opponent casts an
//! artifact, instant, or sorcery spell, counter that spell. This ability triggers only once each turn."

use super::*;

pub const HANDLERS: &[Handler] =
    &[Handler::effect("JinGitaxiasProgressTyrant.JinGitaxiasProgressTyrantEffect", apply)];

/// the spell the trigger saw (`getValue("spellCast")`), countered while it's still on the stack
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    if game.obj(spell).zone != Zone::Stack {
        return Some(());
    }
    cx.run_on(game, d, &[EffectDef::CounterTargetEffect], vec![Target::Obj(spell)]);
    Some(())
}
