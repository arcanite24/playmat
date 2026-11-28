//! Kheru Spellsnatcher: "Morph {4}{U}{U}. When Kheru Spellsnatcher is turned face up, counter target spell. If that
//! spell is countered this way, exile it instead of putting it into its owner's graveyard. You may cast that card
//! without paying its mana cost for as long as it remains exiled."

use super::*;

pub const HANDLERS: &[Handler] = &[Handler::effect("KheruSpellsnatcher.KheruSpellsnatcherEffect", apply)];

/// `counter(…, PutCards.EXILED)`; countered and not a copy: `exileAndPlayFromExile(YOU, Duration.Custom,
/// withoutMana, onlyCast)`, cast by you for free while it stays exiled
fn apply(game: &mut Game, _d: &mut dyn Decider, cx: &EffectCx) -> Option<()> {
    let Some(o) = cx.obj(0) else { return Some(()) };
    if game.obj(o).zone != Zone::Stack || cant_be_countered(game, o) {
        return Some(());
    }
    note_countered(game, o, cx.item.card, cx.controller());
    Arc::make_mut(&mut game.stack).retain(|i| i.card != o || i.ability.is_some() || i.delayed.is_some());
    game.move_to(o, Zone::Exile);
    if game.obj(o).zone != Zone::Exile || game.obj(o).token {
        return Some(());
    }
    let you = cx.controller();
    let ob = game.obj_mut(o);
    ob.may_play = Some((you, u32::MAX));
    ob.may_play_end_step = false;
    ob.may_play_while = None;
    ob.play_free = true;
    ob.cast_only = true;
    Some(())
}
