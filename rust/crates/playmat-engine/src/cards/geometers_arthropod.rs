//! Geometer's Arthropod: "Whenever you cast a spell with {X} in its mana cost, look at the top X cards of your
//! library. Put one of them into your hand and the rest on the bottom of your library in a random order."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("GeometersArthropod.GeometersArthropodEffect", apply)
    .effects(&["new LookLibraryAndPickControllerEffect(1, 1, PutCards.HAND, PutCards.BOTTOM_RANDOM)"])];

/// the X the spell was cast with (`getValue("spellCast")`); below 1, nothing; else
/// `LookLibraryAndPickControllerEffect(X, 1, HAND, BOTTOM_RANDOM)`
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(spell) = cx.item.cast_spell else { return Some(()) };
    let x = match game.stack.iter().find(|i| i.card == spell && i.ability.is_none()) {
        Some(i) => i.x,
        None if game.obj(spell).zone == Zone::Battlefield => game.obj(spell).cast_x,
        None => 0,
    };
    if x < 1 {
        return Some(());
    }
    let mut e = cx.effects.first()?.clone();
    if let EffectDef::LookLibraryAndPickControllerEffect { look, .. } = &mut e {
        *look = x;
    }
    cx.run(game, d, &[e]);
    Some(())
}
