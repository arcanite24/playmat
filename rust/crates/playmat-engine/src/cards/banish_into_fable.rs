//! Banish into Fable: "When you cast this spell from your hand, copy it if you control an artifact, then copy it if
//! you control an enchantment. You may choose new targets for the copies. Return target nonland permanent to its
//! owner's hand. You create a 2/2 white Knight creature token with vigilance."

use super::*;

pub const HANDLERS: &[Handler] = &[
    Handler::trigger(
        "BanishIntoFable.BanishIntoFableTriggeredAbility",
        "new CastSourceTriggeredAbility(new BanishIntoFableEffect(null), false)",
        cast_from_hand,
    ),
    Handler::effect("BanishIntoFable.BanishIntoFableEffect", apply),
];

/// `CastFromHandWatcher.spellWasCastFromHand`: the spell (this card on the stack) was cast from its caster's hand
fn cast_from_hand(game: &Game, item: &StackItem) -> bool {
    let spell = item.cast_spell.unwrap_or(item.card);
    game.obj(spell).cast_from == Some(Zone::Hand)
}

/// The spell (`getSpellOrLKIStack`): copied if you control an artifact, then again if you control an enchantment,
/// each copy's targets chosen anew if you like
fn apply(game: &mut Game, d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let you = cx.controller();
    let spell = cx.item.cast_spell.unwrap_or(cx.item.card);
    let Some(original) =
        game.stack.iter().find(|i| i.card == spell && i.ability.is_none() && i.delayed.is_none()).cloned()
    else {
        return Some(());
    };
    let controls = |game: &Game, t: CardType| {
        game.battlefield.iter().any(|o| game.obj(*o).controller == you && game.has_type(*o, t))
    };
    if controls(game, CardType::Artifact) {
        copy_spell_with(game, d, &original, you, true, false)?;
    }
    if controls(game, CardType::Enchantment) {
        copy_spell_with(game, d, &original, you, true, false)?;
    }
    Some(())
}
