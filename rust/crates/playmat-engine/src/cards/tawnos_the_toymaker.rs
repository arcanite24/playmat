//! Tawnos, the Toymaker: "Whenever you cast a Beast or Bird creature spell, you may copy it, except the copy is an
//! artifact in addition to its other types."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("TawnosTheToymaker.TawnosTheToymakerEffect", apply)];

/// `spell.createCopyOnStack(game, source, you, false, 1, TawnosTheToymakerApplier)`: one copy, controlled by you, no
/// new targets asked, an artifact in addition to its other types (`addCardType(ARTIFACT)`)
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    let copy = copy_spell_with(game, d, &original, you, false, false)?;
    let def = Arc::make_mut(&mut game.obj_mut(copy).def);
    if !def.types.contains(&CardType::Artifact) {
        def.types.push(CardType::Artifact);
    }
    Some(())
}
